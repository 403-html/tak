//! RFC 0016 price-based parser for the unchanged RFC 0014 `TKLZ` syntax.
//!
//! The parser changes only encoder choices. Every block it emits is a valid
//! RFC 0014 block, so the existing decoder restores the output.

use tak_context::{ByteModel, Match8Model};

use super::{
    AdaptiveDistribution, Block, Entry, MAX_DISTANCE, MAX_MATCH, MIN_MATCH, TABLE_LEN, insert,
    key_at, table_index,
};

/// Maximum number of positions in one dynamic-programming segment.
const SEGMENT_LEN: usize = 65_536;
/// A candidate of at least this length is taken immediately.
const NICE_LENGTH: usize = 1_024;
/// Every truncated length in `MIN_MATCH..MIN_MATCH + SHORT_LENGTHS` is priced.
const SHORT_LENGTHS: usize = 64;
const SYMBOLS: usize = 32;

/// One table candidate at a segment position.
#[derive(Clone, Copy, Default)]
struct Candidate {
    length: u32,
    distance: u32,
}

/// Static prices, in bits, taken from the coder state at segment start.
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

/// Mirror of the encoder's adaptive metadata distributions.
struct CoderState {
    types: AdaptiveDistribution,
    literal_lengths: AdaptiveDistribution,
    match_lengths: AdaptiveDistribution,
    distances: AdaptiveDistribution,
}

/// Emits blocks in encoder order and keeps the mirrored coder state current.
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

    fn finish(mut self) -> Vec<Block> {
        self.flush_literal();
        self.blocks
    }
}

/// Parses `input` with segment-wise minimum-price dynamic programming.
pub(super) fn parse_priced(input: &[u8]) -> Vec<Block> {
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut model = Match8Model::new();
    let mut emitter = Emitter::new();
    let mut candidates: Vec<[Candidate; 2]> = Vec::with_capacity(SEGMENT_LEN);
    let mut literal_bits: Vec<f64> = Vec::with_capacity(SEGMENT_LEN);
    let mut position = 0;
    while position < input.len() {
        let start = position;
        candidates.clear();
        literal_bits.clear();
        let mut forced = None;
        while position < input.len() && position - start < SEGMENT_LEN {
            let found = find_candidates(input, position, &table);
            if found[0].length as usize >= NICE_LENGTH {
                forced = Some(found[0]);
                break;
            }
            candidates.push(found);
            literal_bits.push(model_bits(&model, input[position]));
            insert(input, position, &mut table);
            model.update(input[position]);
            position += 1;
        }
        solve_segment(start, &candidates, &literal_bits, &mut emitter);
        if let Some(candidate) = forced {
            let length = candidate.length as usize;
            emitter.matched(length, candidate.distance as usize);
            for consumed in position..position + length {
                insert(input, consumed, &mut table);
                model.update(input[consumed]);
            }
            position += length;
        }
    }
    emitter.finish()
}

/// Returns the newest candidate first. The older candidate is kept only when
/// it is strictly longer. A length of zero marks an absent candidate.
fn find_candidates(input: &[u8], position: usize, table: &[Entry]) -> [Candidate; 2] {
    let mut found = [Candidate::default(); 2];
    let Some(key) = key_at(input, position) else {
        return found;
    };
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return found;
    }
    if let Some(newest) = candidate_at(input, position, entry.position) {
        found[0] = newest;
    }
    if entry.has_previous {
        if let Some(older) = candidate_at(input, position, entry.previous_position) {
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
    found
}

fn candidate_at(input: &[u8], position: usize, candidate: u32) -> Option<Candidate> {
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
    Some(Candidate {
        length: u32::try_from(length).expect("match length is bounded"),
        distance: u32::try_from(distance).expect("distance is bounded"),
    })
}

fn model_bits(model: &Match8Model, byte: u8) -> f64 {
    let (low, high, total) = model.range(byte);
    f64::from(total).log2() - f64::from(high - low).log2()
}

/// How a dynamic-programming state was reached.
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

/// Chooses the minimum-price block sequence for one segment and emits it.
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
    use super::{Block, parse_priced};

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
