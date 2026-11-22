use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_order2_artifact, decode_order3_artifact, encode_order2_artifact, encode_order3_artifact,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-order3-gate: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_path = args
        .first()
        .map(PathBuf::from)
        .ok_or("input path is required")?;
    let input = fs::read(&input_path).map_err(|error| error.to_string())?;

    let start = Instant::now();
    let order2 = encode_order2_artifact(&input);
    let order2_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let order2_output =
        decode_order2_artifact(&order2, input.len() as u64).map_err(|error| error.to_string())?;
    let order2_decode_seconds = start.elapsed().as_secs_f64();

    let start = Instant::now();
    let order3 = encode_order3_artifact(&input);
    let order3_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let order3_output =
        decode_order3_artifact(&order3, input.len() as u64).map_err(|error| error.to_string())?;
    let order3_decode_seconds = start.elapsed().as_secs_f64();

    if order2_output != input || order3_output != input {
        return Err("round-trip mismatch".to_owned());
    }
    if let Some(path) = value(&args, "--output") {
        if PathBuf::from(path).exists() {
            return Err(format!("refusing to overwrite {path}"));
        }
        fs::write(path, &order3).map_err(|error| error.to_string())?;
    }
    println!(
        "input_bytes\torder2_bytes\torder3_bytes\torder2_encode_seconds\torder2_decode_seconds\torder3_encode_seconds\torder3_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{order2_encode_seconds:.6}\t{order2_decode_seconds:.6}\t{order3_encode_seconds:.6}\t{order3_decode_seconds:.6}\ttrue",
        input.len(),
        order2.len(),
        order3.len()
    );
    Ok(())
}

fn value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| argument == name)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}
