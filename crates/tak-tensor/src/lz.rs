use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom, Write};

use tak_context::{ByteModel, Match8Model};

use crate::{HEADER_LEN, crc32, read_u64};

mod priced;

const MAGIC: &[u8; 4] = b"TKLZ";
const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;
const MIN_MATCH: usize = 8;
const MAX_MATCH: usize = 65_535;
const MAX_DISTANCE: usize = 64 * 1024 * 1024;
const MAX_DISTANCE_U64: u64 = 64 * 1024 * 1024;
const INPUT_PAGE_LEN: usize = 64 * 1024;
const INPUT_PAGE_COUNT: usize = 4;
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

/// Byte counts produced by bounded RFC 0014 streaming decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LzStreamStats {
    /// Restored bytes written to the output.
    pub output_bytes: u64,
    /// Arithmetic payload bytes consumed from the artifact.
    pub payload_bytes: u64,
}

/// Byte counts produced by bounded RFC 0014 streaming encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LzEncodeStats {
    /// Source bytes read from the input.
    pub input_bytes: u64,
    /// Arithmetic payload bytes written after the artifact header.
    pub payload_bytes: u64,
    /// Complete artifact bytes written to the output.
    pub artifact_bytes: u64,
}

/// Errors produced by bounded RFC 0014 streaming decode.
#[derive(Debug)]
pub enum LzStreamError {
    /// The artifact is invalid.
    Artifact(LzError),
    /// The source exceeds the experimental encoder-position limit.
    InputTooLarge,
    /// Reading or writing failed.
    Io(io::Error),
}

impl std::fmt::Display for LzStreamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(error) => error.fmt(formatter),
            Self::InputTooLarge => formatter.write_str("TKLZ input exceeds u32 positions"),
            Self::Io(error) => write!(formatter, "TKLZ stream I/O failed: {error}"),
        }
    }
}

impl std::error::Error for LzStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Artifact(error) => Some(error),
            Self::InputTooLarge => None,
            Self::Io(error) => Some(error),
        }
    }
}

impl From<io::Error> for LzStreamError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<LzError> for LzStreamError {
    fn from(error: LzError) -> Self {
        Self::Artifact(error)
    }
}

/// Encodes a complete RFC 0014 research artifact.
#[must_use]
pub fn encode_lz_artifact(input: &[u8]) -> Vec<u8> {
    encode_lz_artifact_with_candidates(input, 1)
}

/// Encodes a complete RFC 0015 two-candidate research artifact.
#[must_use]
pub fn encode_lz_two_candidate_artifact(input: &[u8]) -> Vec<u8> {
    encode_lz_artifact_with_candidates(input, 2)
}

/// Encodes a complete RFC 0016 price-based parser research artifact.
///
/// The artifact uses the unchanged RFC 0014 `TKLZ` syntax and decoder.
#[must_use]
pub fn encode_lz_priced_artifact(input: &[u8]) -> Vec<u8> {
    encode_blocks(input, priced::parse_priced(input))
}

/// Encodes an RFC 0016 price-based parser artifact with bounded memory.
///
/// The artifact is byte-identical to [`encode_lz_priced_artifact`].
///
/// # Errors
///
/// Returns an error for I/O failure, a nonempty output, or an input larger than
/// the experimental format's `u32` encoder-position limit.
pub fn encode_lz_priced_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    priced::encode_priced_stream(&mut input, &mut output)
}

fn encode_lz_artifact_with_candidates(input: &[u8], candidates: usize) -> Vec<u8> {
    encode_blocks(input, parse(input, candidates))
}

fn encode_blocks(input: &[u8], blocks: Vec<Block>) -> Vec<u8> {
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

fn stream_crc(input: &mut (impl Read + Seek), input_len: u64) -> Result<Crc32, LzStreamError> {
    input.seek(SeekFrom::Start(0))?;
    let mut crc = Crc32::new();
    let mut buffer = vec![0_u8; INPUT_PAGE_LEN];
    let mut remaining = input_len;
    while remaining != 0 {
        let wanted = usize::try_from(remaining.min(INPUT_PAGE_LEN as u64))
            .map_err(|_| LzError::InvalidArtifact)?;
        input.read_exact(&mut buffer[..wanted])?;
        crc.update_slice(&buffer[..wanted]);
        remaining -= u64::try_from(wanted).map_err(|_| LzError::InvalidArtifact)?;
    }
    Ok(crc)
}

/// Encodes an RFC 0014 artifact without retaining the complete input, block
/// list, or arithmetic payload.
///
/// The input and output must be seekable. The encoder uses four 64 KiB input
/// pages in addition to the fixed match and probability-model tables. It
/// patches the artifact header after the payload length is known.
///
/// # Errors
///
/// Returns an error for I/O failure or an input larger than the experimental
/// format's `u32` encoder-position limit.
pub fn encode_lz_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    encode_lz_stream_with_candidates(&mut input, &mut output, 1)
}

/// Encodes an RFC 0015 two-candidate artifact with bounded memory.
///
/// # Errors
///
/// Returns an error for I/O failure, a nonempty output, or an input larger than
/// the experimental format's `u32` encoder-position limit.
pub fn encode_lz_two_candidate_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    encode_lz_stream_with_candidates(&mut input, &mut output, 2)
}

fn encode_lz_stream_with_candidates(
    input: &mut (impl Read + Seek),
    output: &mut (impl Write + Seek),
    candidates: usize,
) -> Result<LzEncodeStats, LzStreamError> {
    let input_len = input.seek(SeekFrom::End(0))?;
    if input_len > u64::from(u32::MAX) {
        return Err(LzStreamError::InputTooLarge);
    }
    let crc = stream_crc(input, input_len)?;

    if output.seek(SeekFrom::End(0))? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "TKLZ streaming output must be empty",
        )
        .into());
    }
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&[0_u8; HEADER_LEN])?;
    let mut encoder = StreamRangeEncoder::new(output);
    let mut source = PageCache::new(input, input_len);
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut literal_start = 0_u64;
    let mut literal_length = 0_usize;
    let mut position = 0_u64;

    while position < input_len {
        if let Some((length, distance)) =
            find_match_stream(&mut source, position, input_len, &table, candidates)?
        {
            if literal_length != 0 {
                encode_literal_stream(
                    &mut encoder,
                    &mut source,
                    &mut model,
                    &mut types,
                    &mut literal_lengths,
                    literal_start,
                    literal_length,
                )?;
                literal_length = 0;
            }
            encode_adaptive_streaming(&mut encoder, &mut types, 1)?;
            encode_bucket_streaming(&mut encoder, &mut match_lengths, length - MIN_MATCH)?;
            encode_bucket_streaming(&mut encoder, &mut distances, distance - 1)?;
            for offset in 0..length {
                let offset = u64::try_from(offset).map_err(|_| LzError::InvalidArtifact)?;
                let byte = source
                    .byte_at(position + offset)?
                    .ok_or(LzError::InvalidArtifact)?;
                model.update(byte);
                insert_stream(&mut source, position + offset, &mut table)?;
            }
            position += u64::try_from(length).map_err(|_| LzError::InvalidArtifact)?;
            literal_start = position;
        } else {
            if literal_length == 0 {
                literal_start = position;
            }
            literal_length = literal_length
                .checked_add(1)
                .ok_or(LzError::InvalidArtifact)?;
            insert_stream(&mut source, position, &mut table)?;
            position += 1;
        }
    }
    if literal_length != 0 {
        encode_literal_stream(
            &mut encoder,
            &mut source,
            &mut model,
            &mut types,
            &mut literal_lengths,
            literal_start,
            literal_length,
        )?;
    }
    encode_adaptive_streaming(&mut encoder, &mut types, 2)?;
    let (output, payload_len) = encoder.finish()?;
    let artifact_len = (HEADER_LEN as u64)
        .checked_add(payload_len)
        .ok_or(LzError::InvalidArtifact)?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(MAGIC)?;
    output.write_all(&[0; 4])?;
    output.write_all(&input_len.to_le_bytes())?;
    output.write_all(&payload_len.to_le_bytes())?;
    output.write_all(&crc.finish().to_le_bytes())?;
    output.flush()?;

    Ok(LzEncodeStats {
        input_bytes: input_len,
        payload_bytes: payload_len,
        artifact_bytes: artifact_len,
    })
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

/// Decodes an RFC 0014 artifact with bounded input and output buffers.
///
/// The decoder retains at most the 64 MiB match window plus a 64 KiB output
/// buffer. It does not retain the complete artifact or restored output.
///
/// # Errors
///
/// Returns an error for I/O failure, invalid fields, excessive output, invalid
/// arithmetic, invalid match extents, or a checksum mismatch.
pub fn decode_lz_stream(
    mut artifact: impl Read + Seek,
    output: impl Write,
    max_output: u64,
) -> Result<LzStreamStats, LzStreamError> {
    let artifact_len = artifact.seek(SeekFrom::End(0))?;
    artifact.seek(SeekFrom::Start(0))?;
    let mut header = [0_u8; HEADER_LEN];
    artifact.read_exact(&mut header).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            LzStreamError::Artifact(LzError::InvalidArtifact)
        } else {
            LzStreamError::Io(error)
        }
    })?;
    if &header[..4] != MAGIC || header[4..8] != [0; 4] {
        return Err(LzError::InvalidArtifact.into());
    }
    let output_len = read_u64(&header[8..16]);
    if output_len > max_output {
        return Err(LzError::OutputLimitExceeded.into());
    }
    let payload_len = read_u64(&header[16..24]);
    if artifact_len
        != 28_u64
            .checked_add(payload_len)
            .ok_or(LzError::InvalidArtifact)?
    {
        return Err(LzError::InvalidArtifact.into());
    }
    let expected_crc = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    let window_capacity =
        usize::try_from(output_len.min(MAX_DISTANCE_U64)).map_err(|_| LzError::InvalidArtifact)?;
    let mut restored = StreamOutput::new(output, window_capacity);
    let mut decoder = StreamRangeDecoder::new(artifact.take(payload_len))?;
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);

    loop {
        let block_type = decode_adaptive_stream(&mut decoder, &mut types)?;
        match block_type {
            0 => {
                let encoded = decode_bucket_stream(&mut decoder, &mut literal_lengths)?;
                let length = encoded.checked_add(1).ok_or(LzError::InvalidArtifact)?;
                ensure_remaining_u64(restored.bytes, length, output_len)?;
                for _ in 0..length {
                    let scaled = decoder.scaled(model.total())?;
                    let (byte, low, high, total) = model.symbol(scaled);
                    decoder.advance(low, high, total)?;
                    restored.push(byte)?;
                    model.update(byte);
                }
            }
            1 => {
                let encoded_length = decode_bucket_stream(&mut decoder, &mut match_lengths)?;
                let length = encoded_length
                    .checked_add(MIN_MATCH)
                    .filter(|&length| length <= MAX_MATCH)
                    .ok_or(LzError::InvalidArtifact)?;
                let encoded_distance = decode_bucket_stream(&mut decoder, &mut distances)?;
                let distance = encoded_distance
                    .checked_add(1)
                    .filter(|&distance| {
                        distance <= MAX_DISTANCE
                            && u64::try_from(distance).is_ok_and(|value| value <= restored.bytes)
                    })
                    .ok_or(LzError::InvalidArtifact)?;
                ensure_remaining_u64(restored.bytes, length, output_len)?;
                for _ in 0..length {
                    let byte = restored.match_byte(distance)?;
                    restored.push(byte)?;
                    model.update(byte);
                }
            }
            2 => {
                if restored.bytes != output_len {
                    return Err(LzError::InvalidArtifact.into());
                }
                break;
            }
            _ => return Err(LzError::InvalidArtifact.into()),
        }
    }
    let (bytes, actual_crc) = restored.finish()?;
    if actual_crc != expected_crc {
        return Err(LzError::ChecksumMismatch.into());
    }
    Ok(LzStreamStats {
        output_bytes: bytes,
        payload_bytes: payload_len,
    })
}

fn ensure_remaining_u64(position: u64, length: usize, output_len: u64) -> Result<(), LzError> {
    let length = u64::try_from(length).map_err(|_| LzError::InvalidArtifact)?;
    if position
        .checked_add(length)
        .filter(|&end| end <= output_len)
        .is_none()
    {
        return Err(LzError::InvalidArtifact);
    }
    Ok(())
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

struct PageCache<R> {
    input: R,
    input_len: u64,
    pages: Vec<CachedPage>,
    tick: u64,
}

struct CachedPage {
    index: u64,
    bytes: Vec<u8>,
    last_used: u64,
}

impl<R: Read + Seek> PageCache<R> {
    fn new(input: R, input_len: u64) -> Self {
        Self {
            input,
            input_len,
            pages: Vec::with_capacity(INPUT_PAGE_COUNT),
            tick: 0,
        }
    }

    fn byte_at(&mut self, position: u64) -> io::Result<Option<u8>> {
        if position >= self.input_len {
            return Ok(None);
        }
        let page_index = position / INPUT_PAGE_LEN as u64;
        let offset = usize::try_from(position % INPUT_PAGE_LEN as u64)
            .map_err(|_| io::Error::other("TKLZ input page offset exceeds usize"))?;
        self.tick = self.tick.wrapping_add(1);
        if let Some(page) = self.pages.iter_mut().find(|page| page.index == page_index) {
            page.last_used = self.tick;
            return Ok(page.bytes.get(offset).copied());
        }

        let page = {
            let start = page_index * INPUT_PAGE_LEN as u64;
            let length = usize::try_from((self.input_len - start).min(INPUT_PAGE_LEN as u64))
                .map_err(|_| io::Error::other("TKLZ input page length exceeds usize"))?;
            let mut bytes = vec![0_u8; length];
            self.input.seek(SeekFrom::Start(start))?;
            self.input.read_exact(&mut bytes)?;
            CachedPage {
                index: page_index,
                bytes,
                last_used: self.tick,
            }
        };
        if self.pages.len() < INPUT_PAGE_COUNT {
            self.pages.push(page);
        } else {
            let oldest = self
                .pages
                .iter()
                .enumerate()
                .min_by_key(|(_, page)| page.last_used)
                .map(|(index, _)| index)
                .expect("page cache is not empty");
            self.pages[oldest] = page;
        }
        Ok(self
            .pages
            .iter()
            .find(|page| page.index == page_index)
            .and_then(|page| page.bytes.get(offset))
            .copied())
    }
}

fn key_at_stream<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
) -> Result<Option<u64>, LzStreamError> {
    let mut bytes = [0_u8; MIN_MATCH];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        let offset = u64::try_from(offset).map_err(|_| LzError::InvalidArtifact)?;
        let Some(value) = input.byte_at(position + offset)? else {
            return Ok(None);
        };
        *byte = value;
    }
    Ok(Some(u64::from_be_bytes(bytes)))
}

fn insert_stream<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
    table: &mut [Entry],
) -> Result<(), LzStreamError> {
    let Some(key) = key_at_stream(input, position)? else {
        return Ok(());
    };
    let entry = &mut table[table_index(key)];
    let position = u32::try_from(position).map_err(|_| LzError::InvalidArtifact)?;
    if entry.occupied && entry.key == key {
        entry.previous_position = entry.position;
        entry.has_previous = true;
        entry.position = position;
    } else {
        *entry = Entry {
            key,
            position,
            previous_position: 0,
            has_previous: false,
            occupied: true,
        };
    }
    Ok(())
}

fn find_match_stream<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
    input_len: u64,
    table: &[Entry],
    candidates: usize,
) -> Result<Option<(usize, usize)>, LzStreamError> {
    let Some(key) = key_at_stream(input, position)? else {
        return Ok(None);
    };
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return Ok(None);
    }
    let mut best = stream_match_at(input, position, input_len, entry.position)?;
    if candidates == 2 && entry.has_previous {
        if let Some(older) = stream_match_at(input, position, input_len, entry.previous_position)? {
            if best.is_none_or(|latest| older.0 > latest.0) {
                best = Some(older);
            }
        }
    }
    Ok(best)
}

fn stream_match_at<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
    input_len: u64,
    candidate: u32,
) -> Result<Option<(usize, usize)>, LzStreamError> {
    let candidate = u64::from(candidate);
    let Some(distance) = position
        .checked_sub(candidate)
        .filter(|&value| value != 0 && value <= MAX_DISTANCE_U64)
    else {
        return Ok(None);
    };
    let limit = usize::try_from((input_len - position).min(MAX_MATCH as u64))
        .map_err(|_| LzError::InvalidArtifact)?;
    let mut length = MIN_MATCH;
    while length < limit {
        let offset = u64::try_from(length).map_err(|_| LzError::InvalidArtifact)?;
        if input.byte_at(candidate + offset)? != input.byte_at(position + offset)? {
            break;
        }
        length += 1;
    }
    Ok(Some((
        length,
        usize::try_from(distance).map_err(|_| LzError::InvalidArtifact)?,
    )))
}

#[allow(clippy::too_many_arguments)]
fn encode_literal_stream<W: Write, R: Read + Seek>(
    encoder: &mut StreamRangeEncoder<W>,
    input: &mut PageCache<R>,
    model: &mut Match8Model,
    types: &mut AdaptiveDistribution,
    literal_lengths: &mut AdaptiveDistribution,
    start: u64,
    length: usize,
) -> Result<(), LzStreamError> {
    encode_adaptive_streaming(encoder, types, 0)?;
    encode_bucket_streaming(encoder, literal_lengths, length - 1)?;
    for offset in 0..length {
        let offset = u64::try_from(offset).map_err(|_| LzError::InvalidArtifact)?;
        let byte = input
            .byte_at(start + offset)?
            .ok_or(LzError::InvalidArtifact)?;
        let (low, high, total) = model.range(byte);
        encoder.encode(low, high, total)?;
        model.update(byte);
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    position: u32,
    previous_position: u32,
    has_previous: bool,
    occupied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Block {
    Literal { start: usize, length: usize },
    Match { length: usize, distance: usize },
}

fn parse(input: &[u8], candidates: usize) -> Vec<Block> {
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut blocks = Vec::new();
    let mut literal_start = 0;
    let mut literal_length = 0;
    let mut position = 0;
    while position < input.len() {
        if let Some((length, distance)) = find_match(input, position, &table, candidates) {
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

fn find_match(
    input: &[u8],
    position: usize,
    table: &[Entry],
    candidates: usize,
) -> Option<(usize, usize)> {
    let key = key_at(input, position)?;
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return None;
    }
    let mut best = buffered_match_at(input, position, entry.position);
    if candidates == 2 && entry.has_previous {
        if let Some(older) = buffered_match_at(input, position, entry.previous_position) {
            if best.is_none_or(|latest| older.0 > latest.0) {
                best = Some(older);
            }
        }
    }
    best
}

fn buffered_match_at(input: &[u8], position: usize, candidate: u32) -> Option<(usize, usize)> {
    let candidate = usize::try_from(candidate).expect("input position fits usize");
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
    let entry = &mut table[table_index(key)];
    let position = u32::try_from(position).expect("encoder input is bounded to u32");
    if entry.occupied && entry.key == key {
        entry.previous_position = entry.position;
        entry.has_previous = true;
        entry.position = position;
    } else {
        *entry = Entry {
            key,
            position,
            previous_position: 0,
            has_previous: false,
            occupied: true,
        };
    }
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

fn encode_adaptive_streaming<W: Write>(
    encoder: &mut StreamRangeEncoder<W>,
    distribution: &mut AdaptiveDistribution,
    symbol: usize,
) -> io::Result<()> {
    let (low, high, total) = distribution.range(symbol);
    encoder.encode(low, high, total)?;
    distribution.update(symbol);
    Ok(())
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

fn encode_bucket_streaming<W: Write>(
    encoder: &mut StreamRangeEncoder<W>,
    distribution: &mut AdaptiveDistribution,
    value: usize,
) -> io::Result<()> {
    let shifted_value = value + 1;
    let bucket = usize::try_from(usize::BITS - 1 - shifted_value.leading_zeros())
        .expect("bucket fits usize");
    encode_adaptive_streaming(encoder, distribution, bucket)?;
    let base = (1_usize << bucket) - 1;
    let remainder = value - base;
    for shift in (0..bucket).rev() {
        encoder.encode_bit(((remainder >> shift) & 1) != 0)?;
    }
    Ok(())
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

struct StreamRangeEncoder<W> {
    writer: StreamBitWriter<W>,
    low: u64,
    high: u64,
    pending: usize,
}

impl<W: Write> StreamRangeEncoder<W> {
    fn new(output: W) -> Self {
        Self {
            writer: StreamBitWriter::new(output),
            low: 0,
            high: RANGE_MAX,
            pending: 0,
        }
    }

    fn encode(&mut self, cumulative_low: u32, cumulative_high: u32, total: u32) -> io::Result<()> {
        debug_assert!(cumulative_low < cumulative_high && cumulative_high <= total);
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        loop {
            if self.high < HALF {
                self.writer.bit_with_pending(false, &mut self.pending)?;
            } else if self.low >= HALF {
                self.writer.bit_with_pending(true, &mut self.pending)?;
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
        Ok(())
    }

    fn encode_bit(&mut self, bit: bool) -> io::Result<()> {
        let low = u32::from(bit);
        self.encode(low, low + 1, 2)
    }

    fn finish(mut self) -> io::Result<(W, u64)> {
        self.pending += 1;
        self.writer
            .bit_with_pending(self.low >= QUARTER, &mut self.pending)?;
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

struct StreamBitWriter<W> {
    output: W,
    buffer: Vec<u8>,
    current: u8,
    used: u8,
    bytes: u64,
}

impl<W: Write> StreamBitWriter<W> {
    fn new(output: W) -> Self {
        Self {
            output,
            buffer: Vec::with_capacity(INPUT_PAGE_LEN),
            current: 0,
            used: 0,
            bytes: 0,
        }
    }

    fn bit(&mut self, bit: bool) -> io::Result<()> {
        self.current = (self.current << 1) | u8::from(bit);
        self.used += 1;
        if self.used == 8 {
            self.buffer.push(self.current);
            self.bytes += 1;
            self.current = 0;
            self.used = 0;
            if self.buffer.len() == self.buffer.capacity() {
                self.flush()?;
            }
        }
        Ok(())
    }

    fn bit_with_pending(&mut self, bit: bool, pending: &mut usize) -> io::Result<()> {
        self.bit(bit)?;
        for _ in 0..*pending {
            self.bit(!bit)?;
        }
        *pending = 0;
        Ok(())
    }

    fn finish(mut self) -> io::Result<(W, u64)> {
        if self.used != 0 {
            self.current <<= 8 - self.used;
            self.buffer.push(self.current);
            self.bytes += 1;
        }
        self.flush()?;
        Ok((self.output, self.bytes))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
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

fn decode_adaptive_stream<R: Read>(
    decoder: &mut StreamRangeDecoder<R>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzStreamError> {
    let scaled = decoder.scaled(distribution.total)?;
    let (symbol, low, high, total) = distribution.symbol(scaled).ok_or(LzError::InvalidPayload)?;
    decoder.advance(low, high, total)?;
    distribution.update(symbol);
    Ok(symbol)
}

fn decode_bucket_stream<R: Read>(
    decoder: &mut StreamRangeDecoder<R>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzStreamError> {
    let bucket = decode_adaptive_stream(decoder, distribution)?;
    if bucket >= 32 {
        return Err(LzError::InvalidArtifact.into());
    }
    let base = (1_usize << bucket) - 1;
    let mut remainder = 0_usize;
    for _ in 0..bucket {
        remainder = (remainder << 1) | usize::from(decoder.decode_bit()?);
    }
    base.checked_add(remainder)
        .ok_or(LzStreamError::Artifact(LzError::InvalidArtifact))
}

struct StreamRangeDecoder<R> {
    reader: StreamBitReader<R>,
    low: u64,
    high: u64,
    code: u64,
}

impl<R: Read> StreamRangeDecoder<R> {
    fn new(input: R) -> Result<Self, io::Error> {
        let mut reader = StreamBitReader::new(input);
        let mut code = 0;
        for _ in 0..32 {
            code = (code << 1) | u64::from(reader.bit()?);
        }
        Ok(Self {
            reader,
            low: 0,
            high: RANGE_MAX,
            code,
        })
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
    ) -> Result<(), LzStreamError> {
        if cumulative_low >= cumulative_high || cumulative_high > total {
            return Err(LzError::InvalidPayload.into());
        }
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        self.renormalize()?;
        Ok(())
    }

    /// Applies every pending RFC 0014 renormalization step at once.
    ///
    /// Equal leading bits of `low` and `high` are the half-interval steps.
    /// After them the leading bits differ, so only quarter steps can follow.
    /// A quarter step equals a one-bit shift followed by inverting bit 31.
    /// The result is identical to applying the single-bit steps in order.
    #[inline]
    fn renormalize(&mut self) -> Result<(), LzStreamError> {
        let matching = (self.low ^ self.high).leading_zeros() - 32;
        if matching != 0 {
            let fill = (1_u64 << matching) - 1;
            self.low = (self.low << matching) & RANGE_MAX;
            self.high = ((self.high << matching) | fill) & RANGE_MAX;
            self.code = ((self.code << matching) & RANGE_MAX) | self.reader.bits(matching)?;
        }
        let straddle = ((self.low & !self.high) << 1) & RANGE_MAX;
        let quarter = (straddle << 32).leading_ones();
        if quarter != 0 {
            let fill = (1_u64 << quarter) - 1;
            self.low = ((self.low << quarter) & RANGE_MAX) ^ HALF;
            self.high = (((self.high << quarter) | fill) & RANGE_MAX) ^ HALF;
            self.code = (((self.code << quarter) & RANGE_MAX) | self.reader.bits(quarter)?) ^ HALF;
        }
        Ok(())
    }

    fn decode_bit(&mut self) -> Result<bool, LzStreamError> {
        let scaled = self.scaled(2)?;
        self.advance(scaled, scaled + 1, 2)?;
        Ok(scaled != 0)
    }
}

struct StreamBitReader<R> {
    input: R,
    buffer: Box<[u8]>,
    position: usize,
    filled: usize,
    current: u64,
    remaining: u32,
}

impl<R: Read> StreamBitReader<R> {
    fn new(input: R) -> Self {
        Self {
            input,
            buffer: vec![0; 64 * 1024].into_boxed_slice(),
            position: 0,
            filled: 0,
            current: 0,
            remaining: 0,
        }
    }

    /// Loads up to eight payload bytes, most significant first. Bytes after
    /// the end of input read as zero.
    fn refill(&mut self) -> io::Result<()> {
        if self.filled - self.position >= 8 {
            let bytes: [u8; 8] = self.buffer[self.position..self.position + 8]
                .try_into()
                .expect("slice has eight bytes");
            self.position += 8;
            self.current = u64::from_be_bytes(bytes);
            self.remaining = 64;
            return Ok(());
        }
        let byte = self.next_byte()?;
        self.current = u64::from(byte) << 56;
        self.remaining = 8;
        Ok(())
    }

    /// Returns the next payload byte, or zero after the end of input.
    fn next_byte(&mut self) -> io::Result<u8> {
        if self.position == self.filled {
            self.position = 0;
            self.filled = loop {
                match self.input.read(&mut self.buffer) {
                    Ok(count) => break count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            };
            if self.filled == 0 {
                return Ok(0);
            }
        }
        let byte = self.buffer[self.position];
        self.position += 1;
        Ok(byte)
    }

    /// Returns the next `count` payload bits, most significant first.
    #[inline]
    fn bits(&mut self, count: u32) -> io::Result<u64> {
        debug_assert!(count <= 32);
        if count <= self.remaining {
            let value = self.current.checked_shr(64 - count).unwrap_or(0);
            self.current = self.current.checked_shl(count).unwrap_or(0);
            self.remaining -= count;
            return Ok(value);
        }
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u64::from(self.bit()?);
        }
        Ok(value)
    }

    #[inline]
    fn bit(&mut self) -> io::Result<u8> {
        if self.remaining == 0 {
            self.refill()?;
        }
        let bit = u8::try_from(self.current >> 63).expect("one bit fits u8");
        self.current <<= 1;
        self.remaining -= 1;
        Ok(bit)
    }
}

struct StreamOutput<W> {
    output: W,
    buffer: Vec<u8>,
    window: VecDeque<u8>,
    window_capacity: usize,
    crc: Crc32,
    bytes: u64,
}

impl<W: Write> StreamOutput<W> {
    fn new(output: W, window_capacity: usize) -> Self {
        Self {
            output,
            buffer: Vec::with_capacity(64 * 1024),
            window: VecDeque::with_capacity(window_capacity),
            window_capacity,
            crc: Crc32::new(),
            bytes: 0,
        }
    }

    fn push(&mut self, byte: u8) -> io::Result<()> {
        self.buffer.push(byte);
        self.crc.update(byte);
        if self.window.len() == self.window_capacity && self.window_capacity != 0 {
            self.window.pop_front();
        }
        if self.window_capacity != 0 {
            self.window.push_back(byte);
        }
        self.bytes = self
            .bytes
            .checked_add(1)
            .ok_or_else(|| io::Error::other("TKLZ output length exceeds u64"))?;
        if self.buffer.len() == self.buffer.capacity() {
            self.flush()?;
        }
        Ok(())
    }

    fn match_byte(&self, distance: usize) -> Result<u8, LzError> {
        let index = self
            .window
            .len()
            .checked_sub(distance)
            .ok_or(LzError::InvalidArtifact)?;
        self.window
            .get(index)
            .copied()
            .ok_or(LzError::InvalidArtifact)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }

    fn finish(mut self) -> io::Result<(u64, u32)> {
        self.flush()?;
        self.output.flush()?;
        Ok((self.bytes, self.crc.finish()))
    }
}

#[derive(Clone, Copy)]
struct Crc32(u32);

impl Crc32 {
    const fn new() -> Self {
        Self(u32::MAX)
    }

    fn update(&mut self, byte: u8) {
        self.0 ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(self.0 & 1);
            self.0 = (self.0 >> 1) ^ (0xedb8_8320 & mask);
        }
    }

    fn update_slice(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.update(byte);
        }
    }

    const fn finish(self) -> u32 {
        !self.0
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        LzError, decode_lz_artifact, decode_lz_stream, encode_lz_artifact,
        encode_lz_priced_artifact, encode_lz_stream, encode_lz_two_candidate_artifact,
        encode_lz_two_candidate_stream,
    };
    use crate::HEADER_LEN;

    #[test]
    fn bulk_renormalization_matches_single_bit_steps() {
        use super::{
            HALF, QUARTER, RANGE_MAX, StreamBitReader, StreamRangeDecoder, THREE_QUARTERS,
        };

        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let payload: Vec<u8> = (0..4_096).map(|_| next().to_le_bytes()[0]).collect();
        for case in 0..20_000 {
            let mut a = next() & RANGE_MAX;
            let mut b = next() & RANGE_MAX;
            if case % 3 == 0 {
                // Force long runs of shared leading bits and quarter steps.
                let shift = next() % 32;
                b = (a & !((1 << shift) - 1)) | (b & ((1 << shift) - 1));
                if case % 2 == 0 {
                    a = (a & !THREE_QUARTERS) | QUARTER;
                    b = (b & !THREE_QUARTERS) | HALF;
                }
            }
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            let code = low + next() % (high - low + 1);
            let offset = usize::try_from(next() % 4_000).unwrap();
            let mut bulk = StreamRangeDecoder {
                reader: StreamBitReader::new(Cursor::new(&payload[offset..])),
                low,
                high,
                code,
            };
            bulk.renormalize().unwrap();

            let mut reader = StreamBitReader::new(Cursor::new(&payload[offset..]));
            let (mut low, mut high, mut code) = (low, high, code);
            loop {
                if high < HALF {
                } else if low >= HALF {
                    low -= HALF;
                    high -= HALF;
                    code -= HALF;
                } else if low >= QUARTER && high < THREE_QUARTERS {
                    low -= QUARTER;
                    high -= QUARTER;
                    code -= QUARTER;
                } else {
                    break;
                }
                low <<= 1;
                high = (high << 1) | 1;
                code = ((code << 1) & RANGE_MAX) | u64::from(reader.bit().unwrap());
            }
            assert_eq!(
                (bulk.low, bulk.high, bulk.code),
                (low, high, code),
                "case {case}"
            );
            assert_eq!(
                bulk.reader.bit().unwrap(),
                reader.bit().unwrap(),
                "case {case}"
            );
        }
    }

    #[test]
    fn priced_artifact_round_trips_through_existing_decoders() {
        let mut input = Vec::new();
        for round in 0..3_000_u32 {
            input.extend_from_slice(b"price-based parsing keeps TKLZ ");
            input.extend_from_slice(&(round % 97).to_le_bytes());
        }
        input.extend(std::iter::repeat_n(7_u8, 5_000));
        let artifact = encode_lz_priced_artifact(&input);
        let limit = input.len() as u64;
        assert_eq!(decode_lz_artifact(&artifact, limit).unwrap(), input);
        let mut restored = Vec::new();
        decode_lz_stream(Cursor::new(&artifact), &mut restored, limit).unwrap();
        assert_eq!(restored, input);
    }

    #[test]
    fn priced_artifact_handles_empty_and_tiny_inputs() {
        for input in [&b""[..], b"a", b"abcdefg", b"abcdefgh"] {
            let artifact = encode_lz_priced_artifact(input);
            assert_eq!(decode_lz_artifact(&artifact, 8).unwrap(), input);
        }
    }

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

    #[test]
    fn streaming_decode_matches_buffered_decode() {
        let input = b"abcdefgh01234567abcdefgh01234567".repeat(5_000);
        let artifact = encode_lz_artifact(&input);
        let mut output = Vec::new();
        let stats = decode_lz_stream(
            Cursor::new(&artifact),
            &mut output,
            u64::try_from(input.len()).unwrap(),
        )
        .unwrap();
        assert_eq!(output, input);
        assert_eq!(stats.output_bytes, u64::try_from(input.len()).unwrap());
        assert_eq!(
            stats.payload_bytes,
            u64::try_from(artifact.len() - HEADER_LEN).unwrap()
        );

        let mut rejected = Vec::new();
        assert!(decode_lz_stream(Cursor::new(&artifact), &mut rejected, 1).is_err());
    }

    #[test]
    fn streaming_encode_is_byte_identical() {
        let cases = [
            Vec::new(),
            b"short input".to_vec(),
            b"abcdefgh01234567abcdefgh01234567".repeat(5_000),
        ];
        for input in cases {
            let expected = encode_lz_artifact(&input);
            let mut output = Cursor::new(Vec::new());
            let stats = encode_lz_stream(Cursor::new(&input), &mut output).unwrap();
            assert_eq!(output.into_inner(), expected);
            assert_eq!(stats.input_bytes, u64::try_from(input.len()).unwrap());
            assert_eq!(stats.artifact_bytes, u64::try_from(expected.len()).unwrap());
        }

        let mut nonempty = Cursor::new(vec![0_u8]);
        assert!(encode_lz_stream(Cursor::new(b"input"), &mut nonempty).is_err());
    }

    #[test]
    fn two_candidate_artifacts_round_trip_and_stream_identically() {
        let input = b"abcdefgh22222222abcdefgh11111111abcdefgh22222222".repeat(1_000);
        let artifact = encode_lz_two_candidate_artifact(&input);
        assert_eq!(
            decode_lz_artifact(&artifact, u64::try_from(input.len()).unwrap()).unwrap(),
            input
        );

        let mut streamed = Cursor::new(Vec::new());
        encode_lz_two_candidate_stream(Cursor::new(&input), &mut streamed).unwrap();
        assert_eq!(streamed.into_inner(), artifact);
    }
}
