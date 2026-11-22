use std::{env, fs, num::NonZeroU16, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_order2_artifact, decode_order3_backoff_artifact, encode_order2_artifact,
    encode_order3_backoff_artifact,
};

const THRESHOLDS: [u16; 6] = [2, 4, 8, 16, 32, 64];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-order3-backoff-gate: error: {error}");
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
    let thresholds = value(&args, "--threshold")
        .map(|value| {
            value
                .parse::<u16>()
                .ok()
                .and_then(NonZeroU16::new)
                .map(|threshold| vec![threshold.get()])
                .ok_or_else(|| "threshold must be an integer from 1 to 65535".to_owned())
        })
        .transpose()?
        .unwrap_or_else(|| THRESHOLDS.to_vec());
    let input = fs::read(input_path).map_err(|error| error.to_string())?;

    let start = Instant::now();
    let order2 = encode_order2_artifact(&input);
    let order2_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let restored =
        decode_order2_artifact(&order2, input.len() as u64).map_err(|error| error.to_string())?;
    let order2_decode_seconds = start.elapsed().as_secs_f64();
    if restored != input {
        return Err("order-2 round-trip mismatch".to_owned());
    }

    println!(
        "threshold\tinput_bytes\torder2_bytes\tbackoff_bytes\torder2_encode_seconds\torder2_decode_seconds\tbackoff_encode_seconds\tbackoff_decode_seconds\troundtrip"
    );
    for threshold in thresholds {
        let threshold = NonZeroU16::new(threshold).expect("screening thresholds are nonzero");
        let start = Instant::now();
        let artifact = encode_order3_backoff_artifact(&input, threshold);
        let encode_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let restored = decode_order3_backoff_artifact(&artifact, input.len() as u64)
            .map_err(|error| error.to_string())?;
        let decode_seconds = start.elapsed().as_secs_f64();
        if restored != input {
            return Err(format!("threshold {} round-trip mismatch", threshold.get()));
        }
        println!(
            "{}\t{}\t{}\t{}\t{order2_encode_seconds:.6}\t{order2_decode_seconds:.6}\t{encode_seconds:.6}\t{decode_seconds:.6}\ttrue",
            threshold.get(),
            input.len(),
            order2.len(),
            artifact.len()
        );
    }
    Ok(())
}

fn value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| argument == name)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}
