use std::{env, fs, path::PathBuf, process::ExitCode};

use tak_context::{ByteModel, Match8Model};

const MINIMUMS: [u32; 4] = [2, 4, 8, 16];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match-run-diagnostic: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = match args.as_slice() {
        [path] => PathBuf::from(path),
        _ => return Err("expected exactly one input path".to_owned()),
    };
    let input = fs::read(path).map_err(|error| error.to_string())?;
    let input_len = u32::try_from(input.len())
        .map_err(|_| "diagnostic input must not exceed 4294967295 bytes")?;
    let runs = collect_runs(&input);

    println!(
        "minimum_run\tinput_bytes\truns\tcovered_bytes\tcoverage\tmodel_bits\ttoken_bound_bits\tgross_saved_bits\tgross_saved_bytes"
    );
    for minimum in MINIMUMS {
        let selected: Vec<_> = runs.iter().filter(|run| run.length >= minimum).collect();
        let covered: u64 = selected.iter().map(|run| u64::from(run.length)).sum();
        let model_bits: f64 = selected.iter().map(|run| run.model_bits).sum();
        let token_bit_total: u64 = selected
            .iter()
            .map(|run| u64::from(token_bits(run.length)))
            .sum();
        let token_bits_float: f64 = selected
            .iter()
            .map(|run| f64::from(token_bits(run.length)))
            .sum();
        let saved_bits = model_bits - token_bits_float;
        println!(
            "{minimum}\t{input_len}\t{}\t{covered}\t{:.9}\t{model_bits:.3}\t{token_bit_total}\t{saved_bits:.3}\t{:.3}",
            selected.len(),
            ratio(covered, u64::from(input_len)),
            saved_bits / 8.0,
        );
    }
    Ok(())
}

struct Run {
    length: u32,
    model_bits: f64,
}

fn collect_runs(input: &[u8]) -> Vec<Run> {
    let mut model = Match8Model::new();
    let mut runs = Vec::new();
    let mut length = 0_u32;
    let mut model_bits = 0.0;
    for &actual in input {
        let (low, high, total) = model.range(actual);
        let bits = f64::from(total).log2() - f64::from(high - low).log2();
        if model.match_prediction() == Some(actual) {
            length += 1;
            model_bits += bits;
        } else if length != 0 {
            runs.push(Run { length, model_bits });
            length = 0;
            model_bits = 0.0;
        }
        model.update(actual);
    }
    if length != 0 {
        runs.push(Run { length, model_bits });
    }
    runs
}

fn token_bits(length: u32) -> u32 {
    let significant_bits = 32 - length.leading_zeros();
    1 + 8 * significant_bits.div_ceil(7)
}

fn ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let numerator = u32::try_from(numerator).expect("covered bytes do not exceed input bytes");
    let denominator = u32::try_from(denominator).expect("input length is bounded to u32");
    f64::from(numerator) / f64::from(denominator)
}

#[cfg(test)]
mod tests {
    use super::{collect_runs, token_bits};

    #[test]
    fn collects_repeated_prediction_runs() {
        let runs = collect_runs(b"abcdefghabcdefghabcdefgh");
        assert!(runs.iter().any(|run| run.length >= 8));
        assert_eq!(token_bits(4), 9);
        assert_eq!(token_bits(128), 17);
    }
}
