use std::{
    collections::HashMap, env, fs, num::NonZeroU16, path::PathBuf, process::ExitCode, time::Instant,
};

use tak_context::{ByteModel, Order2NormalizedModel};

const BONUS_DIVISORS: [u32; 4] = [1, 4, 16, 64];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match-dependency: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (input_path, tail_bytes) = match args.as_slice() {
        [path] => (PathBuf::from(path), None),
        [path, option, value] if option == "--tail-bytes" => {
            let bytes = value
                .parse::<usize>()
                .map_err(|_| "tail-bytes must be a nonnegative integer")?;
            (PathBuf::from(path), Some(bytes))
        }
        _ => return Err("expected INPUT [--tail-bytes N]".to_owned()),
    };
    let input = fs::read(input_path).map_err(|error| error.to_string())?;
    let selected = if let Some(tail_bytes) = tail_bytes {
        let start = input
            .len()
            .checked_sub(tail_bytes)
            .ok_or("tail-bytes exceeds input length")?;
        &input[start..]
    } else {
        &input
    };
    let input_len = u32::try_from(selected.len())
        .map_err(|_| "diagnostic input must not exceed 4294967295 bytes")?;

    println!(
        "suffix_bytes\tbonus_divisor\tinput_bytes\tmatched_positions\tcorrect_predictions\tmatch_coverage\tmatch_accuracy\tbaseline_bits_per_byte\taugmented_bits_per_byte\tbits_per_byte_reduction\ttotal_bit_reduction\tseconds"
    );
    measure::<4>(selected, input_len);
    measure::<8>(selected, input_len);
    measure::<16>(selected, input_len);
    Ok(())
}

fn measure<const N: usize>(input: &[u8], input_len: u32) {
    let start = Instant::now();
    let diagnostic = diagnose::<N>(input);
    let seconds = start.elapsed().as_secs_f64();
    let input_len_float = f64::from(input_len);
    let coverage = ratio(diagnostic.matched, input_len);
    let accuracy = if diagnostic.matched == 0 {
        0.0
    } else {
        f64::from(diagnostic.correct) / f64::from(diagnostic.matched)
    };
    let baseline_bits_per_byte = if input.is_empty() {
        0.0
    } else {
        diagnostic.baseline_bits / input_len_float
    };
    for (index, divisor) in BONUS_DIVISORS.iter().enumerate() {
        let augmented_bits_per_byte = if input.is_empty() {
            0.0
        } else {
            diagnostic.augmented_bits[index] / input_len_float
        };
        let reduction = diagnostic.baseline_bits - diagnostic.augmented_bits[index];
        println!(
            "{N}\t{divisor}\t{}\t{}\t{}\t{coverage:.9}\t{accuracy:.9}\t{baseline_bits_per_byte:.9}\t{augmented_bits_per_byte:.9}\t{:.9}\t{reduction:.3}\t{seconds:.6}",
            input.len(),
            diagnostic.matched,
            diagnostic.correct,
            baseline_bits_per_byte - augmented_bits_per_byte,
        );
    }
}

struct Diagnostic {
    baseline_bits: f64,
    augmented_bits: [f64; BONUS_DIVISORS.len()],
    matched: u32,
    correct: u32,
}

fn diagnose<const N: usize>(input: &[u8]) -> Diagnostic {
    let strength = NonZeroU16::new(4).expect("four is nonzero");
    let mut baseline = Order2NormalizedModel::new(strength);
    let mut continuations = HashMap::<[u8; N], u8>::new();
    let mut baseline_bits = 0.0_f64;
    let mut augmented_bits = [0.0_f64; BONUS_DIVISORS.len()];
    let mut matched = 0_u32;
    let mut correct = 0_u32;

    for (position, &actual) in input.iter().enumerate() {
        let (low, high, total) = baseline.range(actual);
        let frequency = high - low;
        let symbol_bits = f64::from(total).log2() - f64::from(frequency).log2();
        baseline_bits += symbol_bits;
        for bits in &mut augmented_bits {
            *bits += symbol_bits;
        }

        if position >= N {
            let suffix: [u8; N] = input[position - N..position]
                .try_into()
                .expect("slice length equals suffix length");
            if let Some(&prediction) = continuations.get(&suffix) {
                matched += 1;
                if prediction == actual {
                    correct += 1;
                }
                for (index, divisor) in BONUS_DIVISORS.iter().enumerate() {
                    let bonus = total / divisor;
                    let augmented_total = total + bonus;
                    let augmented_frequency =
                        frequency + if prediction == actual { bonus } else { 0 };
                    let augmented_symbol_bits =
                        f64::from(augmented_total).log2() - f64::from(augmented_frequency).log2();
                    augmented_bits[index] += augmented_symbol_bits - symbol_bits;
                }
            }
            continuations.insert(suffix, actual);
        }
        baseline.update(actual);
    }
    Diagnostic {
        baseline_bits,
        augmented_bits,
        matched,
        correct,
    }
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        f64::from(numerator) / f64::from(denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::diagnose;

    #[test]
    fn exact_suffix_prediction_is_causal() {
        let correct = diagnose::<8>(b"abcdefghXabcdefghX");
        assert_eq!(correct.matched, 1);
        assert_eq!(correct.correct, 1);
        assert!(correct.augmented_bits[1] < correct.baseline_bits);

        let incorrect = diagnose::<8>(b"abcdefghXabcdefghY");
        assert_eq!(incorrect.matched, 1);
        assert_eq!(incorrect.correct, 0);
        assert!(incorrect.augmented_bits[1] > incorrect.baseline_bits);
    }
}
