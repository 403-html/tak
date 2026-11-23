use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_lz_artifact, decode_match8_artifact, encode_lz_artifact, encode_match8_artifact,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-gate: error: {error}");
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
    let limit = u64::try_from(input.len()).map_err(|_| "input length exceeds u64")?;

    let start = Instant::now();
    let baseline = encode_match8_artifact(input);
    let baseline_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let baseline_output =
        decode_match8_artifact(&baseline, limit).map_err(|error| error.to_string())?;
    let baseline_decode_seconds = start.elapsed().as_secs_f64();

    let start = Instant::now();
    let candidate = encode_lz_artifact(input);
    let candidate_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let candidate_output =
        decode_lz_artifact(&candidate, limit).map_err(|error| error.to_string())?;
    let candidate_decode_seconds = start.elapsed().as_secs_f64();
    if baseline_output != input || candidate_output != input {
        return Err("round-trip mismatch".to_owned());
    }
    let baseline_size = u32::try_from(baseline.len()).map_err(|_| "baseline exceeds u32")?;
    let candidate_size = u32::try_from(candidate.len()).map_err(|_| "candidate exceeds u32")?;
    let saved_bytes = i64::from(baseline_size) - i64::from(candidate_size);
    let reduction = 1.0 - f64::from(candidate_size) / f64::from(baseline_size);

    println!(
        "input_bytes\tmatch8_bytes\tlz_bytes\tsaved_bytes\treduction\tmatch8_encode_seconds\tmatch8_decode_seconds\tlz_encode_seconds\tlz_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{}\t{:.9}\t{baseline_encode_seconds:.9}\t{baseline_decode_seconds:.9}\t{candidate_encode_seconds:.9}\t{candidate_decode_seconds:.9}\ttrue",
        input.len(),
        baseline.len(),
        candidate.len(),
        saved_bytes,
        reduction,
    );
    Ok(())
}
