use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_lz_phase_artifact, decode_lz_structural_artifact, encode_lz_phase_artifact,
    encode_lz_structural_artifact,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-structural-gate: error: {error}");
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
    let control = encode_lz_phase_artifact(input);
    let control_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let control_output =
        decode_lz_phase_artifact(&control, limit).map_err(|error| error.to_string())?;
    let control_decode_seconds = start.elapsed().as_secs_f64();

    let start = Instant::now();
    let candidate = encode_lz_structural_artifact(input);
    let candidate_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let candidate_output =
        decode_lz_structural_artifact(&candidate, limit).map_err(|error| error.to_string())?;
    let candidate_decode_seconds = start.elapsed().as_secs_f64();
    if control_output != input || candidate_output != input {
        return Err("round-trip mismatch".to_owned());
    }

    let saved_bytes = i64::try_from(control.len()).map_err(|_| "control too large")?
        - i64::try_from(candidate.len()).map_err(|_| "candidate too large")?;
    let input_bytes = u32::try_from(input.len()).map_err(|_| "input exceeds u32")?;
    let control_bytes = u32::try_from(control.len()).map_err(|_| "control exceeds u32")?;
    let candidate_bytes = u32::try_from(candidate.len()).map_err(|_| "candidate exceeds u32")?;
    let saved_bits_per_byte = if input.is_empty() {
        0.0
    } else {
        8.0 * (f64::from(control_bytes) - f64::from(candidate_bytes)) / f64::from(input_bytes)
    };
    println!(
        "input_bytes\trfc0017_bytes\trfc0019_bytes\tsaved_bytes\tsaved_bits_per_byte\trfc0017_encode_seconds\trfc0017_decode_seconds\trfc0019_encode_seconds\trfc0019_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{saved_bytes}\t{saved_bits_per_byte:.9}\t{control_encode_seconds:.9}\t{control_decode_seconds:.9}\t{candidate_encode_seconds:.9}\t{candidate_decode_seconds:.9}\ttrue",
        input.len(),
        control.len(),
        candidate.len(),
    );
    Ok(())
}
