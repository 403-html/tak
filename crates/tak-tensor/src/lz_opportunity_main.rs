use std::{env, fs, path::PathBuf, process::ExitCode};

use tak_context::{ByteModel, Match8Model};

const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;
const MIN_MATCH: usize = 8;
const MAX_MATCH: usize = 65_535;
const MAX_DISTANCE: usize = 64 * 1024 * 1024;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-opportunity: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (path, tail_bytes) = match args.as_slice() {
        [path] => (PathBuf::from(path), None),
        [path, option, value] if option == "--tail-bytes" => (
            PathBuf::from(path),
            Some(
                value
                    .parse::<usize>()
                    .map_err(|_| "tail-bytes must be a nonnegative integer")?,
            ),
        ),
        _ => return Err("expected INPUT [--tail-bytes N]".to_owned()),
    };
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

    let blocks = parse(input);
    let result = account(input, &blocks);
    println!(
        "input_bytes\tliteral_blocks\tmatch_blocks\tliteral_bytes\tmatch_bytes\tmatch_coverage\tbaseline_bits\ttype_bits\tlength_bits\tdistance_bits\tliteral_bits\tcandidate_bits\tsaved_bits\tsaved_bits_per_byte"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{:.9}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.9}",
        input.len(),
        result.literal_blocks,
        result.match_blocks,
        result.literal_bytes,
        result.match_bytes,
        ratio(result.match_bytes, input.len()),
        result.baseline_bits,
        result.type_bits,
        result.length_bits,
        result.distance_bits,
        result.literal_bits,
        result.candidate_bits(),
        result.saved_bits(),
        divide(result.saved_bits(), input.len()),
    );
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    position: u32,
    occupied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        let found = find_match(input, position, &table);
        if let Some((length, distance)) = found {
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
        position: u32::try_from(position).expect("input is bounded to u32"),
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

#[derive(Default)]
struct Accounting {
    literal_blocks: usize,
    match_blocks: usize,
    literal_bytes: usize,
    match_bytes: usize,
    baseline_bits: f64,
    type_bits: f64,
    length_bits: f64,
    distance_bits: f64,
    literal_bits: f64,
}

impl Accounting {
    fn candidate_bits(&self) -> f64 {
        self.type_bits + self.length_bits + self.distance_bits + self.literal_bits
    }

    fn saved_bits(&self) -> f64 {
        self.baseline_bits - self.candidate_bits()
    }
}

fn account(input: &[u8], blocks: &[Block]) -> Accounting {
    let mut result = Accounting::default();
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut position = 0;
    for block in blocks {
        match *block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                result.literal_blocks += 1;
                result.literal_bytes += length;
                result.type_bits += types.code(0);
                result.length_bits += bucket_bits(length - 1, &mut literal_lengths);
                for &byte in &input[start..start + length] {
                    let bits = model_bits(&model, byte);
                    result.baseline_bits += bits;
                    result.literal_bits += bits;
                    model.update(byte);
                }
                position += length;
            }
            Block::Match { length, distance } => {
                result.match_blocks += 1;
                result.match_bytes += length;
                result.type_bits += types.code(1);
                result.length_bits += bucket_bits(length - MIN_MATCH, &mut match_lengths);
                result.distance_bits += bucket_bits(distance - 1, &mut distances);
                for &byte in &input[position..position + length] {
                    result.baseline_bits += model_bits(&model, byte);
                    model.update(byte);
                }
                position += length;
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

fn bucket_bits(value: usize, distribution: &mut AdaptiveDistribution) -> f64 {
    let encoded = value + 1;
    let bucket =
        usize::try_from(usize::BITS - 1 - encoded.leading_zeros()).expect("bucket fits usize");
    distribution.code(bucket)
        + f64::from(u32::try_from(bucket).expect("bucket is bounded by usize bits"))
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let numerator = u32::try_from(numerator).expect("input is bounded to u32");
    let denominator = u32::try_from(denominator).expect("input is bounded to u32");
    f64::from(numerator) / f64::from(denominator)
}

fn divide(value: f64, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let denominator = u32::try_from(denominator).expect("input is bounded to u32");
    value / f64::from(denominator)
}

#[cfg(test)]
mod tests {
    use super::{AdaptiveDistribution, Block, account, bucket_bits, parse};

    #[test]
    fn repeated_data_creates_a_match_and_complete_accounting() {
        let input = b"abcdefgh01234567abcdefgh01234567";
        let blocks = parse(input);
        assert!(
            blocks
                .iter()
                .any(|block| matches!(block, Block::Match { length, .. } if *length >= 16))
        );
        let result = account(input, &blocks);
        assert_eq!(result.literal_bytes + result.match_bytes, input.len());
        assert!(result.candidate_bits().is_finite());
    }

    #[test]
    fn bucket_code_charges_remainder_width() {
        let mut distribution = AdaptiveDistribution::new(32);
        let zero = bucket_bits(0, &mut distribution);
        let seven = bucket_bits(7, &mut distribution);
        assert!(seven > zero + 2.0);
    }
}
