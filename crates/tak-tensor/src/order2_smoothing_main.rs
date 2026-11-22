use std::{env, fs, num::NonZeroU16, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_order2_artifact, decode_order2_smoothed_artifact, encode_order2_artifact,
    encode_order2_smoothed_artifact,
};

const WEIGHTS: [u16; 7] = [1, 2, 4, 8, 16, 32, 64];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-order2-smoothing-gate: error: {error}");
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
    let weights = value(&args, "--weight")
        .map(|value| {
            value
                .parse::<u16>()
                .ok()
                .filter(|weight| WEIGHTS.contains(weight))
                .map(|weight| vec![weight])
                .ok_or_else(|| "weight must be one of 1, 2, 4, 8, 16, 32, or 64".to_owned())
        })
        .transpose()?
        .unwrap_or_else(|| WEIGHTS.to_vec());
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
        "weight\tinput_bytes\torder2_bytes\tsmoothed_bytes\torder2_encode_seconds\torder2_decode_seconds\tsmoothed_encode_seconds\tsmoothed_decode_seconds\troundtrip"
    );
    for weight in weights {
        let weight = NonZeroU16::new(weight).expect("screening weights are nonzero");
        let start = Instant::now();
        let artifact = encode_order2_smoothed_artifact(&input, weight);
        let encode_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let restored = decode_order2_smoothed_artifact(&artifact, input.len() as u64)
            .map_err(|error| error.to_string())?;
        let decode_seconds = start.elapsed().as_secs_f64();
        if restored != input {
            return Err(format!("weight {} round-trip mismatch", weight.get()));
        }
        println!(
            "{}\t{}\t{}\t{}\t{order2_encode_seconds:.6}\t{order2_decode_seconds:.6}\t{encode_seconds:.6}\t{decode_seconds:.6}\ttrue",
            weight.get(),
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
