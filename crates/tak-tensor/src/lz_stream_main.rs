use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::{env, process::ExitCode};

use tak_tensor::{
    decode_lz_stream, encode_lz_priced_artifact, encode_lz_priced_stream, encode_lz_stream,
    encode_lz_two_candidate_stream,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-stream: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, input, output] if command == "encode" => encode(input, output),
        [command, input, output] if command == "encode-two" => encode_two(input, output),
        [command, input, output] if command == "encode-priced" => encode_priced(input, output),
        [command, input, output] if command == "encode-priced-bounded" => {
            encode_priced_bounded(input, output)
        }
        [command, input, output, option, limit]
            if command == "decode" && option == "--max-output" =>
        {
            let limit = limit
                .parse::<u64>()
                .map_err(|_| "max-output must be a nonnegative integer")?;
            decode(input, output, limit)
        }
        _ => {
            Err(
                "expected encode INPUT OUTPUT, encode-two INPUT OUTPUT, encode-priced INPUT OUTPUT, encode-priced-bounded INPUT OUTPUT, or decode INPUT OUTPUT --max-output BYTES"
                    .to_owned(),
            )
        }
    }
}

fn encode_two(input: &str, output: &str) -> Result<(), String> {
    require_new_output(output)?;
    let input = File::open(input).map_err(|error| error.to_string())?;
    let output = create_new(output)?;
    encode_lz_two_candidate_stream(input, output)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Encodes with the RFC 0016 price-based parser.
///
/// This research path reads the complete input. It is not a bounded encoder.
fn encode_priced(input: &str, output: &str) -> Result<(), String> {
    require_new_output(output)?;
    let input = std::fs::read(input).map_err(|error| error.to_string())?;
    if u32::try_from(input.len()).is_err() {
        return Err("input must not exceed 4294967295 bytes".to_owned());
    }
    let artifact = encode_lz_priced_artifact(&input);
    let mut output = create_new(output)?;
    output
        .write_all(&artifact)
        .and_then(|()| output.flush())
        .map_err(|error| error.to_string())
}

/// Encodes with the RFC 0016 price-based parser and bounded memory.
fn encode_priced_bounded(input: &str, output: &str) -> Result<(), String> {
    require_new_output(output)?;
    let input = File::open(input).map_err(|error| error.to_string())?;
    let output = create_new(output)?;
    encode_lz_priced_stream(input, output)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn encode(input: &str, output: &str) -> Result<(), String> {
    require_new_output(output)?;
    let input = File::open(input).map_err(|error| error.to_string())?;
    let output = create_new(output)?;
    encode_lz_stream(input, output)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn decode(input: &str, output: &str, max_output: u64) -> Result<(), String> {
    require_new_output(output)?;
    let input = File::open(input).map_err(|error| error.to_string())?;
    let output = create_new(output)?;
    decode_lz_stream(input, output, max_output)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn require_new_output(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        return Err("output already exists".to_owned());
    }
    Ok(())
}

fn create_new(path: &str) -> Result<BufWriter<File>, String> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    Ok(BufWriter::new(file))
}
