use std::{env, fs, path::PathBuf, process::ExitCode};

const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match-reliability: error: {error}");
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
    let input_len = u32::try_from(input.len())
        .map_err(|_| "diagnostic input must not exceed 4294967295 bytes")?;
    let result = diagnose(input);
    let global_bpe = divide(result.global_bits, result.matched);
    let feature_bpe = divide(result.feature_bits, result.matched);
    let suffix_bpe = divide(result.suffix_bits, result.matched);
    println!(
        "input_bytes\tmatched_events\tcorrect_events\taccuracy\tglobal_bits_per_event\tfeature_bits_per_event\tfeature_reduction_bits_per_event\tsuffix_bits_per_event\tsuffix_reduction_bits_per_event"
    );
    println!(
        "{input_len}\t{}\t{}\t{:.9}\t{global_bpe:.9}\t{feature_bpe:.9}\t{:.9}\t{suffix_bpe:.9}\t{:.9}",
        result.matched,
        result.correct,
        ratio(result.correct, result.matched),
        global_bpe - feature_bpe,
        feature_bpe - suffix_bpe,
    );
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    continuation: u8,
    position: u32,
    occupied: bool,
    reliability: Counts,
}

#[derive(Clone, Copy)]
struct Counts {
    correct: u32,
    incorrect: u32,
}

impl Default for Counts {
    fn default() -> Self {
        Self {
            correct: 1,
            incorrect: 1,
        }
    }
}

impl Counts {
    fn bits(self, correct: bool) -> f64 {
        let total = self.correct + self.incorrect;
        let count = if correct {
            self.correct
        } else {
            self.incorrect
        };
        f64::from(total).log2() - f64::from(count).log2()
    }

    fn update(&mut self, correct: bool) {
        if correct {
            self.correct += 1;
        } else {
            self.incorrect += 1;
        }
    }
}

struct ResultRow {
    matched: u32,
    correct: u32,
    global_bits: f64,
    feature_bits: f64,
    suffix_bits: f64,
}

fn diagnose(input: &[u8]) -> ResultRow {
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut global = Counts::default();
    let mut features = [[Counts::default(); 6]; 4];
    let mut history = 0_u64;
    let mut seen = 0_u8;
    let mut correct_run = 0_u32;
    let mut result = ResultRow {
        matched: 0,
        correct: 0,
        global_bits: 0.0,
        feature_bits: 0.0,
        suffix_bits: 0.0,
    };
    for (position, &actual) in input.iter().enumerate() {
        let position = u32::try_from(position).expect("input length is bounded to u32");
        if seen >= 8 {
            let index = table_index(history);
            let mut entry = table[index];
            if entry.occupied && entry.key == history {
                let is_correct = entry.continuation == actual;
                let distance = position - entry.position;
                let distance_bucket = match distance {
                    0..=255 => 0,
                    256..=4_095 => 1,
                    4_096..=65_535 => 2,
                    _ => 3,
                };
                let run_bucket = match correct_run {
                    0 => 0,
                    1 => 1,
                    2..=3 => 2,
                    4..=7 => 3,
                    8..=15 => 4,
                    _ => 5,
                };
                result.matched += 1;
                result.correct += u32::from(is_correct);
                result.global_bits += global.bits(is_correct);
                result.feature_bits += features[distance_bucket][run_bucket].bits(is_correct);
                result.suffix_bits += entry.reliability.bits(is_correct);
                global.update(is_correct);
                features[distance_bucket][run_bucket].update(is_correct);
                entry.reliability.update(is_correct);
                correct_run = if is_correct { correct_run + 1 } else { 0 };
            } else {
                correct_run = 0;
            }
            if entry.occupied && entry.key == history {
                entry.continuation = actual;
                entry.position = position;
                table[index] = entry;
            } else {
                table[index] = Entry {
                    key: history,
                    continuation: actual,
                    position,
                    occupied: true,
                    reliability: Counts::default(),
                };
            }
        }
        history = (history << 8) | u64::from(actual);
        seen = seen.saturating_add(1).min(8);
    }
    result
}

fn table_index(key: u64) -> usize {
    let value = key.wrapping_mul(HASH_MULTIPLIER) >> (64 - TABLE_BITS);
    usize::try_from(value).expect("20-bit index fits usize")
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        f64::from(numerator) / f64::from(denominator)
    }
}

fn divide(bits: f64, events: u32) -> f64 {
    if events == 0 {
        0.0
    } else {
        bits / f64::from(events)
    }
}

#[cfg(test)]
mod tests {
    use super::diagnose;

    #[test]
    fn repeated_suffixes_create_reliability_events() {
        let result = diagnose(b"abcdefghXabcdefghXabcdefghY");
        assert_eq!(result.matched, 10);
        assert_eq!(result.correct, 9);
    }
}
