use std::io::{self, Read, Seek, SeekFrom, Write};

use tak_context::{ByteModel, Match8Model};

use super::{
    AdaptiveDistribution, Block, BlockByteModel, Entry, HEADER_LEN, LzEncodeStats, LzError,
    LzStreamError, MAGIC, MAX_DISTANCE, MAX_MATCH, MIN_MATCH, StandardMatchModel,
    StreamRangeEncoder, TABLE_LEN, encode_adaptive_streaming, encode_bucket_streaming, key_at,
    stream_crc, table_index,
};

const SEGMENT_LEN: usize = 65_536;
const NICE_LENGTH: usize = 1_024;
const SHORT_LENGTHS: usize = 64;
const SYMBOLS: usize = 32;

#[derive(Clone, Copy, Default)]
struct Candidate {
    length: u32,
    distance: u32,
}

struct Prices {
    types: [f64; 3],
    literal_lengths: [f64; SYMBOLS],
    match_lengths: [f64; SYMBOLS],
    distances: [f64; SYMBOLS],
}

impl Prices {
    fn new(state: &CoderState) -> Self {
        Self {
            types: symbol_prices(&state.types),
            literal_lengths: symbol_prices(&state.literal_lengths),
            match_lengths: symbol_prices(&state.match_lengths),
            distances: symbol_prices(&state.distances),
        }
    }

    fn literal_start(&self) -> f64 {
        self.types[0] + self.literal_lengths[0]
    }

    fn match_cost(&self, length: usize, distance: usize) -> f64 {
        self.types[1]
            + bucket_price(&self.match_lengths, length - MIN_MATCH)
            + bucket_price(&self.distances, distance - 1)
    }
}

fn symbol_prices<const N: usize>(distribution: &AdaptiveDistribution) -> [f64; N] {
    let total = f64::from(distribution.total).log2();
    let mut prices = [0.0; N];
    for (price, &count) in prices.iter_mut().zip(&distribution.counts) {
        *price = total - f64::from(count).log2();
    }
    prices
}

fn bucket_of(value: usize) -> usize {
    let shifted = value + 1;
    usize::try_from(usize::BITS - 1 - shifted.leading_zeros()).expect("bucket fits usize")
}

fn bucket_price(prices: &[f64; SYMBOLS], value: usize) -> f64 {
    let bucket = bucket_of(value);
    prices[bucket] + f64::from(u32::try_from(bucket).expect("bucket is below 64"))
}

struct CoderState {
    types: AdaptiveDistribution,
    literal_lengths: AdaptiveDistribution,
    match_lengths: AdaptiveDistribution,
    distances: AdaptiveDistribution,
}

struct Emitter {
    state: CoderState,
    blocks: Vec<Block>,
    literal_start: usize,
    literal_length: usize,
}

impl Emitter {
    fn new() -> Self {
        Self {
            state: CoderState {
                types: AdaptiveDistribution::new(3),
                literal_lengths: AdaptiveDistribution::new(SYMBOLS),
                match_lengths: AdaptiveDistribution::new(SYMBOLS),
                distances: AdaptiveDistribution::new(SYMBOLS),
            },
            blocks: Vec::new(),
            literal_start: 0,
            literal_length: 0,
        }
    }

    fn literal(&mut self, position: usize) {
        if self.literal_length == 0 {
            self.literal_start = position;
        }
        debug_assert_eq!(self.literal_start + self.literal_length, position);
        self.literal_length += 1;
    }

    fn flush_literal(&mut self) {
        if self.literal_length == 0 {
            return;
        }
        self.state.types.update(0);
        self.state
            .literal_lengths
            .update(bucket_of(self.literal_length - 1));
        self.blocks.push(Block::Literal {
            start: self.literal_start,
            length: self.literal_length,
        });
        self.literal_length = 0;
    }

    fn matched(&mut self, length: usize, distance: usize) {
        self.flush_literal();
        self.state.types.update(1);
        self.state
            .match_lengths
            .update(bucket_of(length - MIN_MATCH));
        self.state.distances.update(bucket_of(distance - 1));
        self.blocks.push(Block::Match { length, distance });
    }
}

pub(super) trait PricedInput {
    fn len(&self) -> usize;

    fn byte(&mut self, position: usize) -> Result<u8, LzStreamError>;

    fn key(&mut self, position: usize) -> Result<Option<u64>, LzStreamError>;

    fn match_length(
        &mut self,
        position: usize,
        candidate: usize,
        limit: usize,
    ) -> Result<usize, LzStreamError>;
}

struct SliceInput<'a>(&'a [u8]);

impl PricedInput for SliceInput<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn byte(&mut self, position: usize) -> Result<u8, LzStreamError> {
        Ok(self.0[position])
    }

    fn key(&mut self, position: usize) -> Result<Option<u64>, LzStreamError> {
        Ok(key_at(self.0, position))
    }

    fn match_length(
        &mut self,
        position: usize,
        candidate: usize,
        limit: usize,
    ) -> Result<usize, LzStreamError> {
        let mut length = MIN_MATCH;
        while length < limit && self.0[candidate + length] == self.0[position + length] {
            length += 1;
        }
        Ok(length)
    }
}

pub(super) fn parse_priced(input: &[u8]) -> Vec<Block> {
    let mut blocks = Vec::new();
    parse_priced_with(&mut SliceInput(input), |_, block| {
        blocks.push(block);
        Ok(())
    })
    .expect("in-memory input cannot fail");
    blocks
}

pub(super) fn parse_priced_with<I: PricedInput>(
    input: &mut I,
    mut sink: impl FnMut(&mut I, Block) -> Result<(), LzStreamError>,
) -> Result<(), LzStreamError> {
    let input_len = input.len();
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut model = Match8Model::new();
    let mut emitter = Emitter::new();
    let mut candidates: Vec<[Candidate; 2]> = Vec::with_capacity(SEGMENT_LEN);
    let mut literal_bits: Vec<f64> = Vec::with_capacity(SEGMENT_LEN);
    let mut position = 0;
    while position < input_len {
        let start = position;
        candidates.clear();
        literal_bits.clear();
        let mut forced = None;
        while position < input_len && position - start < SEGMENT_LEN {
            let key = input.key(position)?;
            let found = find_candidates(input, position, key, &table)?;
            if found[0].length as usize >= NICE_LENGTH {
                forced = Some(found[0]);
                break;
            }
            candidates.push(found);
            let byte = input.byte(position)?;
            literal_bits.push(model_bits(&model, byte));
            insert_key(&mut table, key, position);
            model.update(byte);
            position += 1;
        }
        solve_segment(start, &candidates, &literal_bits, &mut emitter);
        if let Some(candidate) = forced {
            let length = candidate.length as usize;
            emitter.matched(length, candidate.distance as usize);
            for consumed in position..position + length {
                let key = input.key(consumed)?;
                insert_key(&mut table, key, consumed);
                model.update(input.byte(consumed)?);
            }
            position += length;
        }
        for block in emitter.blocks.drain(..) {
            sink(input, block)?;
        }
    }
    emitter.flush_literal();
    for block in emitter.blocks.drain(..) {
        sink(input, block)?;
    }
    Ok(())
}

pub(super) const RING_LEN: usize = 128 * 1024 * 1024;
const READ_CHUNKS_PER_RING: usize = 128;
const OLD_PAGE_LEN: usize = 64 * 1024;

struct WindowInput<R> {
    input: R,
    len: usize,
    ring: Vec<u8>,
    ring_mask: usize,
    read_chunk: usize,
    loaded: usize,
    old_page: Vec<u8>,
    old_page_start: Option<usize>,
}

impl<R: Read + Seek> WindowInput<R> {
    fn new(input: R, len: usize, ring_len: usize) -> Self {
        debug_assert!(ring_len.is_power_of_two() && ring_len >= READ_CHUNKS_PER_RING);
        Self {
            input,
            len,
            ring: vec![0; ring_len],
            ring_mask: ring_len - 1,
            read_chunk: ring_len / READ_CHUNKS_PER_RING,
            loaded: 0,
            old_page: vec![0; OLD_PAGE_LEN],
            old_page_start: None,
        }
    }

    fn ensure(&mut self, end: usize) -> Result<(), LzStreamError> {
        let end = end.min(self.len);
        while self.loaded < end {
            let length = self.read_chunk.min(self.len - self.loaded);
            let offset = self.loaded & self.ring_mask;
            self.input.seek(SeekFrom::Start(self.loaded as u64))?;
            self.input
                .read_exact(&mut self.ring[offset..offset + length])?;
            self.loaded += length;
        }
        Ok(())
    }

    fn in_ring(&self, position: usize) -> bool {
        position < self.loaded && self.loaded - position <= self.ring.len()
    }

    fn old_byte(&mut self, position: usize) -> Result<u8, LzStreamError> {
        let start = position - position % OLD_PAGE_LEN;
        if self.old_page_start != Some(start) {
            let length = OLD_PAGE_LEN.min(self.len - start);
            self.input.seek(SeekFrom::Start(start as u64))?;
            self.input.read_exact(&mut self.old_page[..length])?;
            self.old_page_start = Some(start);
        }
        Ok(self.old_page[position - start])
    }
}

impl<R: Read + Seek> PricedInput for WindowInput<R> {
    fn len(&self) -> usize {
        self.len
    }

    fn byte(&mut self, position: usize) -> Result<u8, LzStreamError> {
        self.ensure(position + 1)?;
        if self.in_ring(position) {
            Ok(self.ring[position & self.ring_mask])
        } else {
            self.old_byte(position)
        }
    }

    fn key(&mut self, position: usize) -> Result<Option<u64>, LzStreamError> {
        if self.len - position < MIN_MATCH {
            return Ok(None);
        }
        let mut key = 0;
        for offset in 0..MIN_MATCH {
            key = (key << 8) | u64::from(self.byte(position + offset)?);
        }
        Ok(Some(key))
    }

    fn match_length(
        &mut self,
        position: usize,
        candidate: usize,
        limit: usize,
    ) -> Result<usize, LzStreamError> {
        self.ensure(position + limit)?;
        debug_assert!(self.in_ring(candidate));
        let mut length = MIN_MATCH;
        while length < limit
            && self.ring[(candidate + length) & self.ring_mask]
                == self.ring[(position + length) & self.ring_mask]
        {
            length += 1;
        }
        Ok(length)
    }
}

pub(super) fn encode_priced_stream(
    input: &mut (impl Read + Seek),
    output: &mut (impl Write + Seek),
) -> Result<LzEncodeStats, LzStreamError> {
    encode_priced_stream_with_model(
        input,
        output,
        RING_LEN,
        *MAGIC,
        StandardMatchModel::new(),
        false,
    )
}

pub(super) fn encode_priced_stream_with_model(
    input: &mut (impl Read + Seek),
    output: &mut (impl Write + Seek),
    ring_len: usize,
    magic: [u8; 4],
    mut model: impl BlockByteModel,
    structural_types: bool,
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

    let len = usize::try_from(input_len).map_err(|_| LzStreamError::InputTooLarge)?;
    let mut source = WindowInput::new(input, len, ring_len);
    let mut encoder = StreamRangeEncoder::new(&mut *output);
    let mut types = AdaptiveDistribution::new(if structural_types { 2 } else { 3 });
    let mut literal_lengths = AdaptiveDistribution::new(SYMBOLS);
    let mut match_lengths = AdaptiveDistribution::new(SYMBOLS);
    let mut distances = AdaptiveDistribution::new(SYMBOLS);
    let mut position = 0_usize;
    let mut previous_was_literal = false;
    parse_priced_with(&mut source, |source, block| {
        match block {
            Block::Literal { start, length } => {
                debug_assert!(!previous_was_literal);
                debug_assert_eq!(start, position);
                encode_adaptive_streaming(&mut encoder, &mut types, 0)?;
                encode_bucket_streaming(&mut encoder, &mut literal_lengths, length - 1)?;
                for offset in 0..length {
                    model.set_phase(Some(super::literal_phase(offset)));
                    let byte = source.byte(start + offset)?;
                    let (low, high, total) = model.range(byte);
                    encoder.encode(low, high, total)?;
                    model.update(byte);
                }
                position += length;
                previous_was_literal = true;
            }
            Block::Match { length, distance } => {
                model.set_phase(None);
                if !structural_types || !previous_was_literal {
                    encode_adaptive_streaming(&mut encoder, &mut types, 1)?;
                }
                encode_bucket_streaming(&mut encoder, &mut match_lengths, length - MIN_MATCH)?;
                encode_bucket_streaming(&mut encoder, &mut distances, distance - 1)?;
                for offset in 0..length {
                    model.update(source.byte(position + offset)?);
                }
                position += length;
                previous_was_literal = false;
            }
        }
        Ok(())
    })?;
    if position != len {
        return Err(LzError::InvalidArtifact.into());
    }
    if !structural_types {
        encode_adaptive_streaming(&mut encoder, &mut types, 2)?;
    }
    let (output, payload_len) = encoder.finish()?;
    let artifact_len = (HEADER_LEN as u64)
        .checked_add(payload_len)
        .ok_or(LzError::InvalidArtifact)?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&magic)?;
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

#[cfg(test)]
fn encode_priced_stream_with_ring(
    input: &mut (impl Read + Seek),
    output: &mut (impl Write + Seek),
    ring_len: usize,
) -> Result<LzEncodeStats, LzStreamError> {
    encode_priced_stream_with_model(
        input,
        output,
        ring_len,
        *MAGIC,
        StandardMatchModel::new(),
        false,
    )
}

fn insert_key(table: &mut [Entry], key: Option<u64>, position: usize) {
    let Some(key) = key else {
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

fn find_candidates<I: PricedInput>(
    input: &mut I,
    position: usize,
    key: Option<u64>,
    table: &[Entry],
) -> Result<[Candidate; 2], LzStreamError> {
    let mut found = [Candidate::default(); 2];
    let Some(key) = key else {
        return Ok(found);
    };
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return Ok(found);
    }
    if let Some(newest) = candidate_at(input, position, entry.position)? {
        found[0] = newest;
    }
    if entry.has_previous {
        if let Some(older) = candidate_at(input, position, entry.previous_position)? {
            if older.length > found[0].length {
                if found[0].length == 0 {
                    found[0] = older;
                } else {
                    found[1] = older;
                }
            }
        }
    }
    if found[1].length > found[0].length {
        found.swap(0, 1);
    }
    Ok(found)
}

fn candidate_at<I: PricedInput>(
    input: &mut I,
    position: usize,
    candidate: u32,
) -> Result<Option<Candidate>, LzStreamError> {
    let candidate = usize::try_from(candidate).expect("input position fits usize");
    let Some(distance) = position
        .checked_sub(candidate)
        .filter(|&distance| distance != 0 && distance <= MAX_DISTANCE)
    else {
        return Ok(None);
    };
    let limit = MAX_MATCH.min(input.len() - position);
    let length = input.match_length(position, candidate, limit)?;
    Ok(Some(Candidate {
        length: u32::try_from(length).expect("match length is bounded"),
        distance: u32::try_from(distance).expect("distance is bounded"),
    }))
}

fn model_bits(model: &Match8Model, byte: u8) -> f64 {
    let (low, high, total) = model.range(byte);
    f64::from(total).log2() - f64::from(high - low).log2()
}

#[derive(Clone, Copy)]
enum Step {
    Unreached,
    Literal {
        from_literal: bool,
    },
    Match {
        from: u32,
        from_literal: bool,
        length: u32,
        distance: u32,
    },
}

fn solve_segment(
    start: usize,
    candidates: &[[Candidate; 2]],
    literal_bits: &[f64],
    emitter: &mut Emitter,
) {
    let count = candidates.len();
    if count == 0 {
        return;
    }
    let prices = Prices::new(&emitter.state);
    let literal_start = prices.literal_start();
    let mut literal_cost = vec![f64::INFINITY; count + 1];
    let mut match_cost = vec![f64::INFINITY; count + 1];
    let mut literal_step = vec![Step::Unreached; count + 1];
    let mut match_step = vec![Step::Unreached; count + 1];
    match_cost[0] = 0.0;
    if emitter.literal_length != 0 {
        literal_cost[0] = 0.0;
    }
    for index in 0..count {
        let bits = literal_bits[index];
        let continued = literal_cost[index] + bits;
        let started = match_cost[index] + bits + literal_start;
        let (cost, from_literal) = if continued <= started {
            (continued, true)
        } else {
            (started, false)
        };
        if cost < literal_cost[index + 1] {
            literal_cost[index + 1] = cost;
            literal_step[index + 1] = Step::Literal { from_literal };
        }

        let (base, from_literal) = if literal_cost[index] < match_cost[index] {
            (literal_cost[index], true)
        } else {
            (match_cost[index], false)
        };
        for candidate in candidates[index] {
            let available = (candidate.length as usize).min(count - index);
            if available < MIN_MATCH {
                continue;
            }
            let distance = candidate.distance as usize;
            let short_end = available.min(MIN_MATCH + SHORT_LENGTHS - 1);
            let lengths =
                (MIN_MATCH..=short_end).chain((available > short_end).then_some(available));
            for length in lengths {
                let cost = base + prices.match_cost(length, distance);
                let end = index + length;
                if cost < match_cost[end] {
                    match_cost[end] = cost;
                    match_step[end] = Step::Match {
                        from: u32::try_from(index).expect("segment index fits u32"),
                        from_literal,
                        length: u32::try_from(length).expect("match length fits u32"),
                        distance: candidate.distance,
                    };
                }
            }
        }
    }

    let mut path = Vec::new();
    let mut index = count;
    let mut in_literal = literal_cost[count] < match_cost[count];
    while index > 0 {
        let step = if in_literal {
            literal_step[index]
        } else {
            match_step[index]
        };
        match step {
            Step::Literal { from_literal } => {
                path.push(step);
                index -= 1;
                in_literal = from_literal;
            }
            Step::Match {
                from, from_literal, ..
            } => {
                path.push(step);
                index = from as usize;
                in_literal = from_literal;
            }
            Step::Unreached => unreachable!("every segment position is reachable by literals"),
        }
    }
    let mut position = start;
    for step in path.into_iter().rev() {
        match step {
            Step::Literal { .. } => {
                emitter.literal(position);
                position += 1;
            }
            Step::Match {
                length, distance, ..
            } => {
                emitter.matched(length as usize, distance as usize);
                position += length as usize;
            }
            Step::Unreached => unreachable!("path contains reached steps only"),
        }
    }
    debug_assert_eq!(position, start + count);
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::super::{decode_lz_artifact, encode_lz_priced_artifact};
    use super::{Block, encode_priced_stream, encode_priced_stream_with_ring, parse_priced};

    fn noise(length: usize, mut state: u64) -> Vec<u8> {
        (0..length)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state.to_le_bytes()[0]
            })
            .collect()
    }

    #[test]
    fn bounded_encoder_matches_buffered_artifact() {
        let mut input = Vec::new();
        for round in 0..2_000_u32 {
            input.extend_from_slice(b"bounded priced parsing ");
            input.extend_from_slice(&(round % 61).to_le_bytes());
        }
        input.extend(noise(70_000, 7));
        input.extend_from_slice(&input.clone()[..90_000]);
        let mut output = Cursor::new(Vec::new());
        let stats = encode_priced_stream(&mut Cursor::new(&input), &mut output).unwrap();
        let artifact = output.into_inner();
        assert_eq!(artifact, encode_lz_priced_artifact(&input));
        assert_eq!(stats.artifact_bytes, artifact.len() as u64);
        assert_eq!(
            decode_lz_artifact(&artifact, input.len() as u64).unwrap(),
            input
        );
    }

    #[test]
    fn literal_runs_older_than_the_ring_are_reread() {
        let mut input = noise(300_000, 11);
        let tail = input[299_000..299_600].to_vec();
        input.extend_from_slice(&tail);
        input.extend_from_slice(&tail);
        let mut output = Cursor::new(Vec::new());
        encode_priced_stream_with_ring(&mut Cursor::new(&input), &mut output, 64 * 1024).unwrap();
        let artifact = output.into_inner();
        assert_eq!(artifact, encode_lz_priced_artifact(&input));
        assert_eq!(
            decode_lz_artifact(&artifact, input.len() as u64).unwrap(),
            input
        );
    }

    #[test]
    fn bounded_encoder_handles_empty_and_tiny_inputs() {
        for input in [&b""[..], b"a", b"abcdefgh"] {
            let mut output = Cursor::new(Vec::new());
            encode_priced_stream(&mut Cursor::new(input), &mut output).unwrap();
            assert_eq!(output.into_inner(), encode_lz_priced_artifact(input));
        }
    }

    fn covered(blocks: &[Block]) -> usize {
        blocks
            .iter()
            .map(|block| match *block {
                Block::Literal { length, .. } | Block::Match { length, .. } => length,
            })
            .sum()
    }

    #[test]
    fn priced_parse_covers_input_in_order() {
        let mut input = Vec::new();
        for round in 0..200_u32 {
            input.extend_from_slice(b"the quick brown fox jumps over ");
            input.extend_from_slice(&round.to_le_bytes());
        }
        let blocks = parse_priced(&input);
        assert_eq!(covered(&blocks), input.len());
        let mut position = 0;
        for block in &blocks {
            match *block {
                Block::Literal { start, length } => {
                    assert_eq!(start, position);
                    position += length;
                }
                Block::Match { length, distance } => {
                    assert!(length >= 8 && distance <= position);
                    assert_eq!(
                        input[position - distance..position - distance + length],
                        input[position..position + length]
                    );
                    position += length;
                }
            }
        }
        assert!(
            blocks
                .iter()
                .any(|block| matches!(block, Block::Match { .. }))
        );
    }

    #[test]
    fn long_runs_cross_segments_and_nice_matches() {
        let mut input = vec![0_u8; 200_000];
        input.extend((0..100_000_u32).map(|value| (value.wrapping_mul(2_654_435_761) >> 24) as u8));
        input.extend_from_slice(&input.clone()[200_000..260_000]);
        let blocks = parse_priced(&input);
        assert_eq!(covered(&blocks), input.len());
    }
}
