//! RFC 0016 token-cost and parser-miss profile for the RFC 0015 LZ parser.
//!
//! This is an ideal-accounting diagnostic. It does not produce an artifact.

use std::{env, fs, path::PathBuf, process::ExitCode};

use tak_context::{ByteModel, Match8Model};

const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;
const MIN_MATCH: usize = 8;
const MAX_MATCH: usize = 65_535;
const MAX_DISTANCE: usize = 64 * 1024 * 1024;
const REPEAT_SLOTS: usize = 4;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-profile: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let (path, tail_bytes) = parse_args(env::args().skip(1))?;
    let input = fs::read(path).map_err(|error| error.to_string())?;
    let input = if let Some(bytes) = tail_bytes {
        let start = input
            .len()
            .checked_sub(bytes)
            .ok_or("tail-bytes exceeds input length")?;
        &input[start..]
    } else {
        &input
    };
    u32::try_from(input.len()).map_err(|_| "input must not exceed 4294967295 bytes")?;

    let parsed = parse(input);
    let profile = account(input, &parsed);
    let columns = profile.columns(input.len());
    let names: Vec<&str> = columns.iter().map(|(name, _)| *name).collect();
    let values: Vec<&str> = columns.iter().map(|(_, value)| value.as_str()).collect();
    println!("{}", names.join("\t"));
    println!("{}", values.join("\t"));
    Ok(())
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<(PathBuf, Option<usize>), String> {
    let mut path = None;
    let mut tail_bytes = None;
    let mut args = args.peekable();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--tail-bytes" => {
                let value = args.next().ok_or("tail-bytes requires a value")?;
                tail_bytes = Some(
                    value
                        .parse::<usize>()
                        .map_err(|_| "tail-bytes must be a nonnegative integer")?,
                );
            }
            _ if argument.starts_with('-') => return Err(format!("unknown option: {argument}")),
            _ if path.is_none() => path = Some(PathBuf::from(argument)),
            _ => return Err("expected one input path".to_owned()),
        }
    }
    let path = path.ok_or("expected INPUT [--tail-bytes N]")?;
    Ok((path, tail_bytes))
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

/// The frozen RFC 0015 parse plus one-step lazy observations.
struct Parsed {
    blocks: Vec<Block>,
    /// Matches for which the next position offers a strictly longer match.
    lazy_longer: usize,
    /// Sum of `next_length - length` over `lazy_longer` matches.
    lazy_extra_bytes: usize,
}

fn parse(input: &[u8]) -> Parsed {
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut parsed = Parsed {
        blocks: Vec::new(),
        lazy_longer: 0,
        lazy_extra_bytes: 0,
    };
    let mut literal_start = 0;
    let mut literal_length = 0;
    let mut position = 0;
    while position < input.len() {
        if let Some((length, distance)) = find_match(input, position, &table) {
            if literal_length != 0 {
                parsed.blocks.push(Block::Literal {
                    start: literal_start,
                    length: literal_length,
                });
                literal_length = 0;
            }
            parsed.blocks.push(Block::Match { length, distance });
            insert(input, position, &mut table);
            // Observe, without acting on it, what a one-step lazy parser would see.
            if let Some((next_length, _)) = find_match(input, position + 1, &table) {
                if next_length > length {
                    parsed.lazy_longer += 1;
                    parsed.lazy_extra_bytes += next_length - length;
                }
            }
            for consumed in position + 1..position + length {
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
        parsed.blocks.push(Block::Literal {
            start: literal_start,
            length: literal_length,
        });
    }
    parsed
}

fn find_match(input: &[u8], position: usize, table: &[Entry]) -> Option<(usize, usize)> {
    let key = key_at(input, position)?;
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return None;
    }
    let mut best = match_at(input, position, entry.position);
    if entry.has_previous {
        if let Some(older) = match_at(input, position, entry.previous_position) {
            if best.is_none_or(|latest| older.0 > latest.0) {
                best = Some(older);
            }
        }
    }
    best
}

fn match_at(input: &[u8], position: usize, candidate: u32) -> Option<(usize, usize)> {
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
    let position = u32::try_from(position).expect("input is bounded to u32");
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
    counts: Vec<u32>,
    total: u32,
}

impl AdaptiveDistribution {
    fn new(symbols: usize) -> Self {
        Self {
            counts: vec![1; symbols],
            total: u32::try_from(symbols).expect("symbol count fits u32"),
        }
    }

    fn code(&mut self, symbol: usize) -> f64 {
        let bits = f64::from(self.total).log2() - f64::from(self.counts[symbol]).log2();
        self.counts[symbol] += 1;
        self.total += 1;
        bits
    }
}

/// Ideal bucket cost split into adaptive bucket bits and raw remainder bits.
struct BucketCost {
    bucket_bits: f64,
    raw_bits: f64,
}

fn bucket_cost(value: usize, distribution: &mut AdaptiveDistribution) -> BucketCost {
    let encoded = value + 1;
    let bucket =
        usize::try_from(usize::BITS - 1 - encoded.leading_zeros()).expect("bucket fits usize");
    BucketCost {
        bucket_bits: distribution.code(bucket),
        raw_bits: f64::from(u32::try_from(bucket).expect("bucket is bounded by usize bits")),
    }
}

#[derive(Default)]
struct Profile {
    literal_blocks: usize,
    match_blocks: usize,
    literal_bytes: usize,
    match_bytes: usize,
    baseline_bits: f64,
    type_bits: f64,
    literal_length_bucket_bits: f64,
    literal_length_raw_bits: f64,
    match_length_bucket_bits: f64,
    match_length_raw_bits: f64,
    distance_bucket_bits: f64,
    distance_raw_bits: f64,
    literal_bits: f64,
    /// Matches whose distance equals the previous match distance.
    repeat0_matches: usize,
    /// Distance bits currently spent on `repeat0_matches`.
    repeat0_distance_bits: f64,
    /// Matches whose distance equals one of the last four distinct distances.
    repeat4_matches: usize,
    /// Distance bits currently spent on `repeat4_matches`.
    repeat4_distance_bits: f64,
    /// Literal blocks that directly follow a match ended by a mismatch.
    excludable_literals: usize,
    /// Ideal bits saved by removing the known mismatching byte from those literals.
    exclusion_saved_bits: f64,
    /// Excludable literals where the byte model predicted the excluded byte.
    exclusion_predicted: usize,
    lazy_longer: usize,
    lazy_extra_bytes: usize,
    /// Matches whose type, length, and distance bits exceed the byte-model bits
    /// of the bytes they cover.
    unprofitable_matches: usize,
    /// Bytes covered by `unprofitable_matches`.
    unprofitable_match_bytes: usize,
    /// Sum of token bits minus covered byte-model bits over `unprofitable_matches`.
    unprofitable_excess_bits: f64,
}

impl Profile {
    fn candidate_bits(&self) -> f64 {
        self.type_bits
            + self.literal_length_bucket_bits
            + self.literal_length_raw_bits
            + self.match_length_bucket_bits
            + self.match_length_raw_bits
            + self.distance_bucket_bits
            + self.distance_raw_bits
            + self.literal_bits
    }

    fn columns(&self, input_bytes: usize) -> Vec<(&'static str, String)> {
        let candidate = self.candidate_bits();
        vec![
            ("input_bytes", input_bytes.to_string()),
            ("literal_blocks", self.literal_blocks.to_string()),
            ("match_blocks", self.match_blocks.to_string()),
            ("literal_bytes", self.literal_bytes.to_string()),
            ("match_bytes", self.match_bytes.to_string()),
            ("baseline_bits", format!("{:.3}", self.baseline_bits)),
            ("candidate_bits", format!("{candidate:.3}")),
            ("type_bits", format!("{:.3}", self.type_bits)),
            (
                "literal_length_bucket_bits",
                format!("{:.3}", self.literal_length_bucket_bits),
            ),
            (
                "literal_length_raw_bits",
                format!("{:.3}", self.literal_length_raw_bits),
            ),
            (
                "match_length_bucket_bits",
                format!("{:.3}", self.match_length_bucket_bits),
            ),
            (
                "match_length_raw_bits",
                format!("{:.3}", self.match_length_raw_bits),
            ),
            (
                "distance_bucket_bits",
                format!("{:.3}", self.distance_bucket_bits),
            ),
            ("distance_raw_bits", format!("{:.3}", self.distance_raw_bits)),
            ("literal_bits", format!("{:.3}", self.literal_bits)),
            ("repeat0_matches", self.repeat0_matches.to_string()),
            (
                "repeat0_distance_bits",
                format!("{:.3}", self.repeat0_distance_bits),
            ),
            ("repeat4_matches", self.repeat4_matches.to_string()),
            (
                "repeat4_distance_bits",
                format!("{:.3}", self.repeat4_distance_bits),
            ),
            ("excludable_literals", self.excludable_literals.to_string()),
            (
                "exclusion_saved_bits",
                format!("{:.3}", self.exclusion_saved_bits),
            ),
            ("exclusion_predicted", self.exclusion_predicted.to_string()),
            ("lazy_longer", self.lazy_longer.to_string()),
            ("lazy_extra_bytes", self.lazy_extra_bytes.to_string()),
            ("unprofitable_matches", self.unprofitable_matches.to_string()),
            (
                "unprofitable_match_bytes",
                self.unprofitable_match_bytes.to_string(),
            ),
            (
                "unprofitable_excess_bits",
                format!("{:.3}", self.unprofitable_excess_bits),
            ),
        ]
    }
}

fn account(input: &[u8], parsed: &Parsed) -> Profile {
    let mut result = Profile {
        lazy_longer: parsed.lazy_longer,
        lazy_extra_bytes: parsed.lazy_extra_bytes,
        ..Profile::default()
    };
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut recent: Vec<usize> = Vec::with_capacity(REPEAT_SLOTS);
    // Byte that the previous match would have produced next, when it ended by mismatch.
    let mut excluded: Option<u8> = None;
    let mut position = 0;
    for block in &parsed.blocks {
        match *block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                result.literal_blocks += 1;
                result.literal_bytes += length;
                result.type_bits += types.code(0);
                let cost = bucket_cost(length - 1, &mut literal_lengths);
                result.literal_length_bucket_bits += cost.bucket_bits;
                result.literal_length_raw_bits += cost.raw_bits;
                for (index, &byte) in input[start..start + length].iter().enumerate() {
                    let bits = model_bits(&model, byte);
                    if index == 0 {
                        if let Some(skip) = excluded {
                            debug_assert_ne!(skip, byte);
                            result.excludable_literals += 1;
                            result.exclusion_saved_bits += exclusion_bits(&model, byte, skip);
                            if model.match_prediction() == Some(skip) {
                                result.exclusion_predicted += 1;
                            }
                        }
                    }
                    result.baseline_bits += bits;
                    result.literal_bits += bits;
                    model.update(byte);
                }
                excluded = None;
                position += length;
            }
            Block::Match { length, distance } => {
                result.match_blocks += 1;
                result.match_bytes += length;
                let type_bits = types.code(1);
                result.type_bits += type_bits;
                let cost = bucket_cost(length - MIN_MATCH, &mut match_lengths);
                let mut token_bits = type_bits + cost.bucket_bits + cost.raw_bits;
                result.match_length_bucket_bits += cost.bucket_bits;
                result.match_length_raw_bits += cost.raw_bits;
                let cost = bucket_cost(distance - 1, &mut distances);
                let distance_bits = cost.bucket_bits + cost.raw_bits;
                token_bits += distance_bits;
                result.distance_bucket_bits += cost.bucket_bits;
                result.distance_raw_bits += cost.raw_bits;
                if let Some(slot) = recent.iter().position(|&value| value == distance) {
                    if slot == 0 {
                        result.repeat0_matches += 1;
                        result.repeat0_distance_bits += distance_bits;
                    }
                    result.repeat4_matches += 1;
                    result.repeat4_distance_bits += distance_bits;
                    recent.remove(slot);
                } else if recent.len() == REPEAT_SLOTS {
                    recent.pop();
                }
                recent.insert(0, distance);
                let mut covered_bits = 0.0;
                for &byte in &input[position..position + length] {
                    covered_bits += model_bits(&model, byte);
                    model.update(byte);
                }
                result.baseline_bits += covered_bits;
                if token_bits > covered_bits {
                    result.unprofitable_matches += 1;
                    result.unprofitable_match_bytes += length;
                    result.unprofitable_excess_bits += token_bits - covered_bits;
                }
                position += length;
                let end = position;
                excluded = (length < MAX_MATCH && end < input.len())
                    .then(|| input[end - distance]);
            }
        }
    }
    debug_assert_eq!(position, input.len());
    result.type_bits += types.code(2);
    result
}

fn model_bits(model: &Match8Model, byte: u8) -> f64 {
    let (low, high, total) = model.range(byte);
    f64::from(total).log2() - f64::from(high - low).log2()
}

/// Ideal saving from coding `byte` with `skip` removed from the distribution.
fn exclusion_bits(model: &Match8Model, byte: u8, skip: u8) -> f64 {
    let (_, _, total) = model.range(byte);
    let (low, high, _) = model.range(skip);
    f64::from(total).log2() - f64::from(total - (high - low)).log2()
}

#[cfg(test)]
mod tests {
    use super::{Block, account, parse};

    #[test]
    fn profile_covers_input_and_splits_costs() {
        let input = b"abcdefgh01234567abcdefgh01234567xyzabcdefgh01234567";
        let parsed = parse(input);
        let profile = account(input, &parsed);
        assert_eq!(profile.literal_bytes + profile.match_bytes, input.len());
        assert!(profile.candidate_bits().is_finite());
        assert!(profile.distance_raw_bits > 0.0);
    }

    #[test]
    fn repeated_distance_is_counted() {
        let mut input = Vec::new();
        for round in 0..4u8 {
            input.extend_from_slice(b"abcdefghijklmnop");
            input.push(b'0' + round);
        }
        let parsed = parse(&input);
        let profile = account(&input, &parsed);
        assert!(profile.repeat0_matches >= 1);
        assert!(profile.repeat4_matches >= profile.repeat0_matches);
    }

    #[test]
    fn literal_after_mismatch_is_excludable() {
        let input = b"abcdefghijXabcdefghijY";
        let parsed = parse(input);
        assert!(
            parsed
                .blocks
                .iter()
                .any(|block| matches!(block, Block::Match { .. }))
        );
        let profile = account(input, &parsed);
        assert_eq!(profile.excludable_literals, 1);
        assert!(profile.exclusion_saved_bits > 0.0);
    }
}
