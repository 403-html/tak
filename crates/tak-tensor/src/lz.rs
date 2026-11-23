use tak_context::{ByteModel, Match8Model};

use crate::{HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKLZ";
const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;
const MIN_MATCH: usize = 8;
const MAX_MATCH: usize = 65_535;
const MAX_DISTANCE: usize = 64 * 1024 * 1024;
const MAX_TOTAL: u32 = 16_384;
const RANGE_MAX: u64 = u32::MAX as u64;
const HALF: u64 = 0x8000_0000;
const QUARTER: u64 = 0x4000_0000;
const THREE_QUARTERS: u64 = 0xc000_0000;

/// Errors produced by the RFC 0014 research artifact decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LzError {
    /// The header, extent, block, or integer field is invalid.
    InvalidArtifact,
    /// The declared output exceeds the caller's limit.
    OutputLimitExceeded,
    /// The arithmetic-coded payload contains an invalid interval.
    InvalidPayload,
    /// The restored bytes do not match the stored checksum.
    ChecksumMismatch,
}

impl std::fmt::Display for LzError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArtifact => formatter.write_str("invalid TKLZ artifact"),
            Self::OutputLimitExceeded => formatter.write_str("TKLZ output limit exceeded"),
            Self::InvalidPayload => formatter.write_str("invalid TKLZ arithmetic payload"),
            Self::ChecksumMismatch => formatter.write_str("TKLZ checksum mismatch"),
        }
    }
}

impl std::error::Error for LzError {}

/// Encodes a complete RFC 0014 research artifact.
#[must_use]
pub fn encode_lz_artifact(input: &[u8]) -> Vec<u8> {
    let blocks = parse(input);
    let mut encoder = RangeEncoder::new();
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut position = 0;

    for block in blocks {
        match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                encode_adaptive(&mut encoder, &mut types, 0);
                encode_bucket(&mut encoder, &mut literal_lengths, length - 1);
                for &byte in &input[start..start + length] {
                    let (low, high, total) = model.range(byte);
                    encoder.encode(low, high, total);
                    model.update(byte);
                }
                position += length;
            }
            Block::Match { length, distance } => {
                encode_adaptive(&mut encoder, &mut types, 1);
                encode_bucket(&mut encoder, &mut match_lengths, length - MIN_MATCH);
                encode_bucket(&mut encoder, &mut distances, distance - 1);
                for &byte in &input[position..position + length] {
                    model.update(byte);
                }
                position += length;
            }
        }
    }
    debug_assert_eq!(position, input.len());
    encode_adaptive(&mut encoder, &mut types, 2);
    let payload = encoder.finish();

    let mut artifact = Vec::with_capacity(HEADER_LEN + payload.len());
    artifact.extend_from_slice(MAGIC);
    artifact.extend_from_slice(&[0; 4]);
    artifact.extend_from_slice(&(input.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&crc32(input).to_le_bytes());
    artifact.extend_from_slice(&payload);
    artifact
}

/// Decodes a complete RFC 0014 research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, invalid arithmetic,
/// invalid match extents, or a checksum mismatch.
pub fn decode_lz_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, LzError> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..8] != [0; 4] {
        return Err(LzError::InvalidArtifact);
    }
    let output_len = read_u64(&artifact[8..16]);
    if output_len > max_output {
        return Err(LzError::OutputLimitExceeded);
    }
    let output_len = usize::try_from(output_len).map_err(|_| LzError::InvalidArtifact)?;
    let payload_len =
        usize::try_from(read_u64(&artifact[16..24])).map_err(|_| LzError::InvalidArtifact)?;
    if artifact.len()
        != HEADER_LEN
            .checked_add(payload_len)
            .ok_or(LzError::InvalidArtifact)?
    {
        return Err(LzError::InvalidArtifact);
    }
    let expected_crc = u32::from_le_bytes([artifact[24], artifact[25], artifact[26], artifact[27]]);
    let mut decoder = RangeDecoder::new(&artifact[HEADER_LEN..]);
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut output = Vec::with_capacity(output_len);

    loop {
        let block_type = decode_adaptive(&mut decoder, &mut types)?;
        match block_type {
            0 => {
                let encoded = decode_bucket(&mut decoder, &mut literal_lengths)?;
                let length = encoded.checked_add(1).ok_or(LzError::InvalidArtifact)?;
                ensure_remaining(output.len(), length, output_len)?;
                for _ in 0..length {
                    let scaled = decoder.scaled(model.total())?;
                    let (byte, low, high, total) = model.symbol(scaled);
                    decoder.advance(low, high, total)?;
                    output.push(byte);
                    model.update(byte);
                }
            }
            1 => {
                let encoded_length = decode_bucket(&mut decoder, &mut match_lengths)?;
                let length = encoded_length
                    .checked_add(MIN_MATCH)
                    .filter(|&length| length <= MAX_MATCH)
                    .ok_or(LzError::InvalidArtifact)?;
                let encoded_distance = decode_bucket(&mut decoder, &mut distances)?;
                let distance = encoded_distance
                    .checked_add(1)
                    .filter(|&distance| distance <= MAX_DISTANCE && distance <= output.len())
                    .ok_or(LzError::InvalidArtifact)?;
                ensure_remaining(output.len(), length, output_len)?;
                for _ in 0..length {
                    let source = output.len() - distance;
                    let byte = output[source];
                    output.push(byte);
                    model.update(byte);
                }
            }
            2 => {
                if output.len() != output_len {
                    return Err(LzError::InvalidArtifact);
                }
                break;
            }
            _ => return Err(LzError::InvalidArtifact),
        }
    }

    if crc32(&output) != expected_crc {
        return Err(LzError::ChecksumMismatch);
    }
    Ok(output)
}

fn ensure_remaining(position: usize, length: usize, output_len: usize) -> Result<(), LzError> {
    if position
        .checked_add(length)
        .filter(|&end| end <= output_len)
        .is_none()
    {
        return Err(LzError::InvalidArtifact);
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    position: u32,
    occupied: bool,
}

#[derive(Clone, Copy)]
enum Block {
    Literal { start: usize, length: usize },
    Match { length: usize, distance: usize },
}

fn parse(input: &[u8]) -> Vec<Block> {
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut blocks = Vec::new();
    let mut literal_start = 0;
    let mut literal_length = 0;
    let mut position = 0;
    while position < input.len() {
        if let Some((length, distance)) = find_match(input, position, &table) {
            if literal_length != 0 {
                blocks.push(Block::Literal {
                    start: literal_start,
                    length: literal_length,
                });
                literal_length = 0;
            }
            blocks.push(Block::Match { length, distance });
            for consumed in position..position + length {
                insert(input, consumed, &mut table);
            }
            position += length;
            literal_start = position;
        } else {
            if literal_length == 0 {
                literal_start = position;
            }
            literal_length += 1;
            insert(input, position, &mut table);
            position += 1;
        }
    }
    if literal_length != 0 {
        blocks.push(Block::Literal {
            start: literal_start,
            length: literal_length,
        });
    }
    blocks
}

fn find_match(input: &[u8], position: usize, table: &[Entry]) -> Option<(usize, usize)> {
    let key = key_at(input, position)?;
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return None;
    }
    let candidate = usize::try_from(entry.position).expect("input position fits usize");
    let distance = position.checked_sub(candidate)?;
    if distance == 0 || distance > MAX_DISTANCE {
        return None;
    }
    let limit = MAX_MATCH.min(input.len() - position);
    let mut length = MIN_MATCH;
    while length < limit && input[candidate + length] == input[position + length] {
        length += 1;
    }
    Some((length, distance))
}

fn insert(input: &[u8], position: usize, table: &mut [Entry]) {
    let Some(key) = key_at(input, position) else {
        return;
    };
    table[table_index(key)] = Entry {
        key,
        position: u32::try_from(position).expect("encoder input is bounded to u32"),
        occupied: true,
    };
}

fn key_at(input: &[u8], position: usize) -> Option<u64> {
    let bytes: [u8; 8] = input.get(position..position + 8)?.try_into().ok()?;
    Some(u64::from_be_bytes(bytes))
}

fn table_index(key: u64) -> usize {
    let value = key.wrapping_mul(HASH_MULTIPLIER) >> (64 - TABLE_BITS);
    usize::try_from(value).expect("20-bit index fits usize")
}

struct AdaptiveDistribution {
    counts: Vec<u16>,
    total: u32,
}

impl AdaptiveDistribution {
    fn new(symbols: usize) -> Self {
        Self {
            counts: vec![1; symbols],
            total: u32::try_from(symbols).expect("symbol count fits u32"),
        }
    }

    fn range(&self, symbol: usize) -> (u32, u32, u32) {
        let low = self.counts[..symbol]
            .iter()
            .map(|&count| u32::from(count))
            .sum();
        (low, low + u32::from(self.counts[symbol]), self.total)
    }

    fn symbol(&self, value: u32) -> Option<(usize, u32, u32, u32)> {
        if value >= self.total {
            return None;
        }
        let mut low = 0;
        for (symbol, &count) in self.counts.iter().enumerate() {
            let high = low + u32::from(count);
            if value < high {
                return Some((symbol, low, high, self.total));
            }
            low = high;
        }
        None
    }

    fn update(&mut self, symbol: usize) {
        if self.total >= MAX_TOTAL {
            self.total = 0;
            for count in &mut self.counts {
                *count = (*count).div_ceil(2);
                self.total += u32::from(*count);
            }
        }
        self.counts[symbol] += 1;
        self.total += 1;
    }
}

fn encode_adaptive(
    encoder: &mut RangeEncoder,
    distribution: &mut AdaptiveDistribution,
    symbol: usize,
) {
    let (low, high, total) = distribution.range(symbol);
    encoder.encode(low, high, total);
    distribution.update(symbol);
}

fn decode_adaptive(
    decoder: &mut RangeDecoder<'_>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzError> {
    let scaled = decoder.scaled(distribution.total)?;
    let (symbol, low, high, total) = distribution.symbol(scaled).ok_or(LzError::InvalidPayload)?;
    decoder.advance(low, high, total)?;
    distribution.update(symbol);
    Ok(symbol)
}

fn encode_bucket(
    encoder: &mut RangeEncoder,
    distribution: &mut AdaptiveDistribution,
    value: usize,
) {
    let shifted_value = value + 1;
    let bucket = usize::try_from(usize::BITS - 1 - shifted_value.leading_zeros())
        .expect("bucket fits usize");
    encode_adaptive(encoder, distribution, bucket);
    let base = (1_usize << bucket) - 1;
    let remainder = value - base;
    for shift in (0..bucket).rev() {
        encoder.encode_bit(((remainder >> shift) & 1) != 0);
    }
}

fn decode_bucket(
    decoder: &mut RangeDecoder<'_>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzError> {
    let bucket = decode_adaptive(decoder, distribution)?;
    if bucket >= 32 {
        return Err(LzError::InvalidArtifact);
    }
    let base = (1_usize << bucket) - 1;
    let mut remainder = 0_usize;
    for _ in 0..bucket {
        remainder = (remainder << 1) | usize::from(decoder.decode_bit()?);
    }
    base.checked_add(remainder).ok_or(LzError::InvalidArtifact)
}

struct RangeEncoder {
    writer: BitWriter,
    low: u64,
    high: u64,
    pending: usize,
}

impl RangeEncoder {
    fn new() -> Self {
        Self {
            writer: BitWriter::default(),
            low: 0,
            high: RANGE_MAX,
            pending: 0,
        }
    }

    fn encode(&mut self, cumulative_low: u32, cumulative_high: u32, total: u32) {
        debug_assert!(cumulative_low < cumulative_high && cumulative_high <= total);
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        loop {
            if self.high < HALF {
                self.writer.bit_with_pending(false, &mut self.pending);
            } else if self.low >= HALF {
                self.writer.bit_with_pending(true, &mut self.pending);
                self.low -= HALF;
                self.high -= HALF;
            } else if self.low >= QUARTER && self.high < THREE_QUARTERS {
                self.pending += 1;
                self.low -= QUARTER;
                self.high -= QUARTER;
            } else {
                break;
            }
            self.low <<= 1;
            self.high = (self.high << 1) | 1;
        }
    }

    fn encode_bit(&mut self, bit: bool) {
        let low = u32::from(bit);
        self.encode(low, low + 1, 2);
    }

    fn finish(mut self) -> Vec<u8> {
        self.pending += 1;
        self.writer
            .bit_with_pending(self.low >= QUARTER, &mut self.pending);
        self.writer.finish()
    }
}

struct RangeDecoder<'a> {
    reader: BitReader<'a>,
    low: u64,
    high: u64,
    code: u64,
}

impl<'a> RangeDecoder<'a> {
    fn new(payload: &'a [u8]) -> Self {
        let mut reader = BitReader::new(payload);
        let mut code = 0;
        for _ in 0..32 {
            code = (code << 1) | u64::from(reader.bit());
        }
        Self {
            reader,
            low: 0,
            high: RANGE_MAX,
            code,
        }
    }

    fn scaled(&self, total: u32) -> Result<u32, LzError> {
        if total == 0 || self.code < self.low || self.code > self.high {
            return Err(LzError::InvalidPayload);
        }
        let width = self.high - self.low + 1;
        let scaled = ((self.code - self.low + 1) * u64::from(total) - 1) / width;
        u32::try_from(scaled).map_err(|_| LzError::InvalidPayload)
    }

    fn advance(
        &mut self,
        cumulative_low: u32,
        cumulative_high: u32,
        total: u32,
    ) -> Result<(), LzError> {
        if cumulative_low >= cumulative_high || cumulative_high > total {
            return Err(LzError::InvalidPayload);
        }
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        loop {
            if self.high < HALF {
            } else if self.low >= HALF {
                self.low -= HALF;
                self.high -= HALF;
                self.code -= HALF;
            } else if self.low >= QUARTER && self.high < THREE_QUARTERS {
                self.low -= QUARTER;
                self.high -= QUARTER;
                self.code -= QUARTER;
            } else {
                break;
            }
            self.low <<= 1;
            self.high = (self.high << 1) | 1;
            self.code = ((self.code << 1) & RANGE_MAX) | u64::from(self.reader.bit());
        }
        Ok(())
    }

    fn decode_bit(&mut self) -> Result<bool, LzError> {
        let scaled = self.scaled(2)?;
        self.advance(scaled, scaled + 1, 2)?;
        Ok(scaled != 0)
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl BitWriter {
    fn bit(&mut self, bit: bool) {
        self.current = (self.current << 1) | u8::from(bit);
        self.used += 1;
        if self.used == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.used = 0;
        }
    }

    fn bit_with_pending(&mut self, bit: bool, pending: &mut usize) {
        self.bit(bit);
        for _ in 0..*pending {
            self.bit(!bit);
        }
        *pending = 0;
    }

    fn finish(mut self) -> Vec<u8> {
        if self.used != 0 {
            self.current <<= 8 - self.used;
            self.bytes.push(self.current);
        }
        self.bytes
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_index: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_index: 0,
        }
    }

    fn bit(&mut self) -> u8 {
        let byte = self.bytes.get(self.bit_index / 8).copied().unwrap_or(0);
        let bit = (byte >> (7 - self.bit_index % 8)) & 1;
        self.bit_index += 1;
        bit
    }
}

#[cfg(test)]
mod tests {
    use super::{LzError, decode_lz_artifact, encode_lz_artifact};

    #[test]
    fn artifacts_round_trip_short_and_repeated_inputs() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefgh01234567abcdefgh01234567".repeat(1_000),
            (0_u8..=255).cycle().take(20_000).collect(),
        ] {
            let artifact = encode_lz_artifact(&input);
            assert_eq!(
                decode_lz_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            if !input.is_empty() {
                assert_eq!(
                    decode_lz_artifact(&artifact, input.len() as u64 - 1),
                    Err(LzError::OutputLimitExceeded)
                );
            }
        }
    }

    #[test]
    fn corruption_and_truncation_are_rejected() {
        let input = b"abcdefgh01234567abcdefgh01234567".repeat(200);
        let artifact = encode_lz_artifact(&input);
        for end in 0..artifact.len() {
            assert!(decode_lz_artifact(&artifact[..end], u64::MAX).is_err());
        }
        for index in [0, 8, 16, 24, artifact.len() / 2] {
            let mut corrupt = artifact.clone();
            corrupt[index] ^= 0x40;
            assert!(decode_lz_artifact(&corrupt, u64::MAX).is_err());
        }
    }
}
