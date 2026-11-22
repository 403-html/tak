use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_codecs::{Method, decode, encode};
use tak_tensor::{
    QuantizedModel, decode_artifact, decode_order2_artifact, encode_artifact,
    encode_order2_artifact,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-tensor-experiment: error: {error}");
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
    let rank = value(&args, "--rank")
        .unwrap_or("2")
        .parse::<usize>()
        .map_err(|_| "invalid rank")?;
    if !(1..=8).contains(&rank) {
        return Err("rank must be between 1 and 8".to_owned());
    }
    let iterations = value(&args, "--iterations")
        .unwrap_or("20")
        .parse::<usize>()
        .map_err(|_| "invalid iterations")?;
    let bytes = fs::read(&input_path).map_err(|error| error.to_string())?;
    let train_bytes = value(&args, "--train-bytes")
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| "invalid training length")
        })
        .transpose()?
        .unwrap_or(bytes.len() * 9 / 10);
    if train_bytes == 0 || train_bytes >= bytes.len() {
        return Err("training length must split the input into nonempty partitions".to_owned());
    }
    let (training, test) = bytes.split_at(train_bytes);
    let start = Instant::now();
    let (model, diagnostics) =
        QuantizedModel::train_with_diagnostics(training, test, rank, iterations);
    let training_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let artifact = encode_artifact(test, &model);
    let encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let restored =
        decode_artifact(&artifact, test.len() as u64).map_err(|error| error.to_string())?;
    let decode_seconds = start.elapsed().as_secs_f64();
    if restored != test {
        return Err("round-trip mismatch".to_owned());
    }
    let start = Instant::now();
    let baseline = encode(test, Method::Order1);
    let order1_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let order1_restored =
        decode(&baseline, test.len() as u64).map_err(|error| error.to_string())?;
    let order1_decode_seconds = start.elapsed().as_secs_f64();
    if order1_restored != test {
        return Err("order-1 control round-trip mismatch".to_owned());
    }
    let start = Instant::now();
    let order2 = encode_order2_artifact(test);
    let order2_encode_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let order2_restored =
        decode_order2_artifact(&order2, test.len() as u64).map_err(|error| error.to_string())?;
    let order2_decode_seconds = start.elapsed().as_secs_f64();
    if order2_restored != test {
        return Err("order-2 control round-trip mismatch".to_owned());
    }
    if let Some(path) = value(&args, "--output") {
        if PathBuf::from(path).exists() {
            return Err(format!("refusing to overwrite {path}"));
        }
        fs::write(path, &artifact).map_err(|error| error.to_string())?;
    }
    println!(
        "rank\ttrain_bytes\ttest_bytes\titerations\tmodel_bytes\tartifact_bytes\torder1_bytes\torder2_bytes\tfull_precision_bpb\tquantized_core_bpb\tinteger_frequency_bpb\ttraining_seconds\tencode_seconds\tdecode_seconds\torder1_encode_seconds\torder1_decode_seconds\torder2_encode_seconds\torder2_decode_seconds\troundtrip"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{training_seconds:.6}\t{encode_seconds:.6}\t{decode_seconds:.6}\t{order1_encode_seconds:.6}\t{order1_decode_seconds:.6}\t{order2_encode_seconds:.6}\t{order2_decode_seconds:.6}\ttrue",
        model.rank(),
        training.len(),
        test.len(),
        iterations,
        model.model_bytes(),
        artifact.len(),
        baseline.len(),
        order2.len(),
        diagnostics.full_precision_bits_per_byte,
        diagnostics.quantized_core_bits_per_byte,
        diagnostics.integer_frequency_bits_per_byte
    );
    Ok(())
}

fn value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|argument| argument == name)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}
