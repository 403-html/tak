use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{decode_lz_artifact, encode_lz_priced_artifact, encode_lz_two_candidate_artifact};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-priced-gate: error: {error}");
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
    let limit = u64::try_from(input.len()).map_err(|_| "input length exceeds u64")?;

    let start = Instant::now();
    let control = encode_lz_two_candidate_artifact(input);
    let control_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let control_output = decode_lz_artifact(&control, limit).map_err(|error| error.to_string())?;
    let control_decode_seconds = start.elapsed().as_secs_f64();

    let start = Instant::now();
    let candidate = encode_lz_priced_artifact(input);
    let candidate_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let candidate_output =
        decode_lz_artifact(&candidate, limit).map_err(|error| error.to_string())?;
    let candidate_decode_seconds = start.elapsed().as_secs_f64();
    if control_output != input || candidate_output != input {
        return Err("round-trip mismatch".to_owned());
    }

    let control_size = u32::try_from(control.len()).map_err(|_| "control exceeds u32")?;
    let candidate_size = u32::try_from(candidate.len()).map_err(|_| "candidate exceeds u32")?;
    let input_size = u32::try_from(input.len()).map_err(|_| "input exceeds u32")?;
    let saved_bytes = i64::from(control_size) - i64::from(candidate_size);
    let saved_bits_per_byte = if input_size == 0 {
        0.0
    } else {
        8.0 * (f64::from(control_size) - f64::from(candidate_size)) / f64::from(input_size)
    };

    println!(
        "input_bytes\trfc0015_bytes\tpriced_bytes\tsaved_bytes\tsaved_bits_per_byte\trfc0015_encode_seconds\trfc0015_decode_seconds\tpriced_encode_seconds\tpriced_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{saved_bytes}\t{saved_bits_per_byte:.9}\t{control_encode_seconds:.9}\t{control_decode_seconds:.9}\t{candidate_encode_seconds:.9}\t{candidate_decode_seconds:.9}\ttrue",
        input.len(),
        control.len(),
        candidate.len(),
    );
    Ok(())
}
