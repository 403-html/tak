use std::{
    env,
    fs::{self, File, OpenOptions},
    num::NonZeroU16,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use tak_tensor::{decode_order2_normalized_stream, encode_order2_normalized_stream};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-order2-normalized-stream: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let operation = args.first().ok_or("operation is required")?;
    let input_path = args
        .get(1)
        .map(PathBuf::from)
        .ok_or("input path is required")?;
    let output_path = args
        .get(2)
        .map(PathBuf::from)
        .ok_or("output path is required")?;
    match operation.as_str() {
        "encode" => encode(&args, &input_path, &output_path),
        "decode" => decode(&args, &input_path, &output_path),
        _ => Err("operation must be encode or decode".to_owned()),
    }
}

fn encode(args: &[String], input_path: &Path, output_path: &Path) -> Result<(), String> {
    let strength = optional_value(args, "--strength")?
        .unwrap_or("4")
        .parse::<u16>()
        .ok()
        .and_then(NonZeroU16::new)
        .filter(|value| [1, 4, 16, 64, 256, 1024].contains(&value.get()))
        .ok_or("strength must be 1, 4, 16, 64, 256, or 1024")?;
    let input = File::open(input_path).map_err(|error| error.to_string())?;
    let output = create_output(output_path)?;
    let start = Instant::now();
    let result = encode_order2_normalized_stream(input, output, strength);
    let seconds = start.elapsed().as_secs_f64();
    match result {
        Ok(stats) => {
            println!("operation\tinput_bytes\tpayload_bytes\tartifact_bytes\tseconds\troundtrip");
            println!(
                "encode\t{}\t{}\t{}\t{seconds:.6}\tunchecked",
                stats.uncompressed_bytes,
                stats.payload_bytes,
                stats.payload_bytes + 28
            );
            Ok(())
        }
        Err(error) => {
            remove_partial(output_path);
            Err(error.to_string())
        }
    }
}

fn decode(args: &[String], input_path: &Path, output_path: &Path) -> Result<(), String> {
    let max_output = optional_value(args, "--max-output")?
        .unwrap_or("18446744073709551615")
        .parse::<u64>()
        .map_err(|_| "max-output must be an integer from 0 to 18446744073709551615")?;
    let input = File::open(input_path).map_err(|error| error.to_string())?;
    let output = create_output(output_path)?;
    let start = Instant::now();
    let result = decode_order2_normalized_stream(input, output, max_output);
    let seconds = start.elapsed().as_secs_f64();
    match result {
        Ok(stats) => {
            println!("operation\toutput_bytes\tpayload_bytes\tseconds\troundtrip");
            println!(
                "decode\t{}\t{}\t{seconds:.6}\tchecksum-verified",
                stats.uncompressed_bytes, stats.payload_bytes
            );
            Ok(())
        }
        Err(error) => {
            remove_partial(output_path);
            Err(error.to_string())
        }
    }
}

fn create_output(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())
}

fn remove_partial(path: &Path) {
    let _ = fs::remove_file(path);
}

fn optional_value<'a>(args: &'a [String], name: &str) -> Result<Option<&'a str>, String> {
    match args {
        [_, _, _] => Ok(None),
        [_, _, _, option, value] if option == name => Ok(Some(value)),
        _ => Err(format!("expected optional {name} VALUE")),
    }
}
