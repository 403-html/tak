use std::{env, fs, num::NonZeroU16, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_match8_artifact, decode_order2_normalized_artifact, encode_match8_artifact,
    encode_order2_normalized_artifact,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match8-gate: error: {error}");
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
    let max_output = u64::try_from(selected.len()).map_err(|_| "input length exceeds u64")?;
    let strength = NonZeroU16::new(4).expect("four is nonzero");

    let start = Instant::now();
    let baseline = encode_order2_normalized_artifact(selected, strength);
    let baseline_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let baseline_output = decode_order2_normalized_artifact(&baseline, max_output)
        .map_err(|error| error.to_string())?;
    let baseline_decode_seconds = start.elapsed().as_secs_f64();

    let start = Instant::now();
    let match8 = encode_match8_artifact(selected);
    let match8_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let match8_output =
        decode_match8_artifact(&match8, max_output).map_err(|error| error.to_string())?;
    let match8_decode_seconds = start.elapsed().as_secs_f64();

    if baseline_output != selected || match8_output != selected {
        return Err("round-trip mismatch".to_owned());
    }
    println!(
        "input_bytes\tbaseline_bytes\tmatch8_bytes\tbaseline_encode_seconds\tbaseline_decode_seconds\tmatch8_encode_seconds\tmatch8_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{baseline_encode_seconds:.6}\t{baseline_decode_seconds:.6}\t{match8_encode_seconds:.6}\t{match8_decode_seconds:.6}\ttrue",
        selected.len(),
        baseline.len(),
        match8.len(),
    );
    Ok(())
}
