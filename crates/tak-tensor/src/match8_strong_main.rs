use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_match8_artifact, decode_match8_strong_artifact, encode_match8_artifact,
    encode_match8_strong_artifact,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match8-strong-gate: error: {error}");
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
    let limit = u64::try_from(input.len()).map_err(|_| "input exceeds u64")?;

    let start = Instant::now();
    let baseline = encode_match8_artifact(&input);
    let baseline_encode = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let baseline_output = decode_match8_artifact(&baseline, limit).map_err(|e| e.to_string())?;
    let baseline_decode = start.elapsed().as_secs_f64();

    let start = Instant::now();
    let strong = encode_match8_strong_artifact(&input);
    let strong_encode = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let strong_output = decode_match8_strong_artifact(&strong, limit).map_err(|e| e.to_string())?;
    let strong_decode = start.elapsed().as_secs_f64();
    if baseline_output != input || strong_output != input {
        return Err("round-trip mismatch".to_owned());
    }
    println!(
        "input_bytes\tmatch8_bytes\tstrong_bytes\tmatch8_encode_seconds\tmatch8_decode_seconds\tstrong_encode_seconds\tstrong_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{baseline_encode:.6}\t{baseline_decode:.6}\t{strong_encode:.6}\t{strong_decode:.6}\ttrue",
        input.len(),
        baseline.len(),
        strong.len()
    );
    Ok(())
}
