use std::{
    env,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use tak_tensor::{decode_match8_stream, encode_match8_stream};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match8-stream: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [operation, input, output] if operation == "encode" => encode(input, output),
        [operation, input, output, option, limit]
            if operation == "decode" && option == "--max-output" =>
        {
            let limit = limit
                .parse::<u64>()
                .map_err(|_| "max-output must be a nonnegative integer")?;
            decode(input, output, limit)
        }
        _ => Err("expected encode INPUT OUTPUT or decode INPUT OUTPUT --max-output N".to_owned()),
    }
}

fn encode(input_path: &str, output_path: &str) -> Result<(), String> {
    let input = File::open(input_path).map_err(|error| error.to_string())?;
    let output = create_output(output_path)?;
    let start = Instant::now();
    match encode_match8_stream(input, output) {
        Ok(stats) => {
            println!("operation\tuncompressed_bytes\tpayload_bytes\tartifact_bytes\tseconds");
            println!(
                "encode\t{}\t{}\t{}\t{:.6}",
                stats.uncompressed_bytes,
                stats.payload_bytes,
                stats.payload_bytes + 28,
                start.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Err(error) => {
            remove_partial(output_path);
            Err(error.to_string())
        }
    }
}

fn decode(input_path: &str, output_path: &str, max_output: u64) -> Result<(), String> {
    let input = File::open(input_path).map_err(|error| error.to_string())?;
    let output = create_output(output_path)?;
    let start = Instant::now();
    match decode_match8_stream(input, output, max_output) {
        Ok(stats) => {
            println!("operation\tuncompressed_bytes\tpayload_bytes\tseconds\troundtrip");
            println!(
                "decode\t{}\t{}\t{:.6}\tchecksum-verified",
                stats.uncompressed_bytes,
                stats.payload_bytes,
                start.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Err(error) => {
            remove_partial(output_path);
            Err(error.to_string())
        }
    }
}

fn create_output(path: impl AsRef<Path>) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())
}

fn remove_partial(path: impl Into<PathBuf>) {
    let _ = fs::remove_file(path.into());
}
