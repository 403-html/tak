use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

use tak_codecs::{Method, decode, encode, encode_auto, inspect};

const DEFAULT_OUTPUT_LIMIT: u64 = 1 << 30;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("tak: error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[OsString]) -> Result<(), String> {
    let Some(command) = args.first().and_then(|value| value.to_str()) else {
        print_help();
        return Ok(());
    };
    match command {
        "compress" => compress(&args[1..]),
        "decompress" => decompress(&args[1..]),
        "verify" => verify(&args[1..]),
        "benchmark" => benchmark(&args[1..]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" => {
            println!("tak {} (experimental format 0)", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(format!("unknown command {command:?}; run `tak help`")),
    }
}

fn compress(args: &[OsString]) -> Result<(), String> {
    let input_path = required_path(args, 0, "compress requires an input file")?;
    let output_path = option_path(args, "-o")?
        .or(option_path(args, "--output")?)
        .unwrap_or_else(|| append_extension(&input_path, "takc"));
    let method = option_value(args, "--method")?
        .map(parse_method)
        .transpose()?;
    let input = fs::read(&input_path).map_err(|error| io_error("read", &input_path, &error))?;
    let frame = method.map_or_else(|| encode_auto(&input), |method| encode(&input, method));
    write_new(&output_path, &frame)?;
    let info = inspect(&frame).map_err(|error| error.to_string())?;
    println!(
        "compressed\tinput_bytes={}\toutput_bytes={}\tmethod={}\tpath={}",
        input.len(),
        frame.len(),
        info.method.name(),
        output_path.display()
    );
    Ok(())
}

fn decompress(args: &[OsString]) -> Result<(), String> {
    let input_path = required_path(args, 0, "decompress requires an input file")?;
    let output_path = option_path(args, "-o")?
        .or(option_path(args, "--output")?)
        .unwrap_or_else(|| remove_takc_extension(&input_path));
    let max_output = option_value(args, "--max-output-bytes")?
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "invalid --max-output-bytes value".to_owned())
        })
        .transpose()?
        .unwrap_or(DEFAULT_OUTPUT_LIMIT);
    let frame = fs::read(&input_path).map_err(|error| io_error("read", &input_path, &error))?;
    let output = decode(&frame, max_output).map_err(|error| error.to_string())?;
    write_new(&output_path, &output)?;
    println!(
        "decompressed\toutput_bytes={}\tpath={}",
        output.len(),
        output_path.display()
    );
    Ok(())
}

fn verify(args: &[OsString]) -> Result<(), String> {
    let input_path = required_path(args, 0, "verify requires an input file")?;
    let max_output = option_value(args, "--max-output-bytes")?
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| "invalid --max-output-bytes value".to_owned())
        })
        .transpose()?
        .unwrap_or(DEFAULT_OUTPUT_LIMIT);
    let frame = fs::read(&input_path).map_err(|error| io_error("read", &input_path, &error))?;
    let info = inspect(&frame).map_err(|error| error.to_string())?;
    decode(&frame, max_output).map_err(|error| error.to_string())?;
    println!(
        "verified\toutput_bytes={}\tmethod={}\tpath={}",
        info.uncompressed_len,
        info.method.name(),
        input_path.display()
    );
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn benchmark(args: &[OsString]) -> Result<(), String> {
    let input_path = required_path(args, 0, "benchmark requires an input file")?;
    let input = fs::read(&input_path).map_err(|error| io_error("read", &input_path, &error))?;
    println!("method\tinput_bytes\toutput_bytes\tratio\tcompress_mib_s\tdecompress_mib_s");
    for method in Method::all() {
        let start = Instant::now();
        let frame = encode(&input, method);
        let compression_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let restored = decode(&frame, input.len() as u64).map_err(|error| error.to_string())?;
        let decompression_seconds = start.elapsed().as_secs_f64();
        if restored != input {
            return Err(format!("{} failed round-trip verification", method.name()));
        }
        let mib = input.len() as f64 / (1024.0 * 1024.0);
        let ratio = if input.is_empty() {
            0.0
        } else {
            frame.len() as f64 / input.len() as f64
        };
        println!(
            "{}\t{}\t{}\t{ratio:.6}\t{:.3}\t{:.3}",
            method.name(),
            input.len(),
            frame.len(),
            throughput(mib, compression_seconds),
            throughput(mib, decompression_seconds)
        );
    }
    eprintln!(
        "note: one in-process sample; use benchmarks/runners/run-baselines.sh for controlled measurements"
    );
    Ok(())
}

fn throughput(mib: f64, seconds: f64) -> f64 {
    if seconds == 0.0 {
        f64::INFINITY
    } else {
        mib / seconds
    }
}

fn required_path(args: &[OsString], index: usize, message: &str) -> Result<PathBuf, String> {
    args.get(index)
        .filter(|value| !value.to_string_lossy().starts_with('-'))
        .map(PathBuf::from)
        .ok_or_else(|| message.to_owned())
}

fn option_value<'a>(args: &'a [OsString], name: &str) -> Result<Option<&'a str>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .and_then(|value| value.to_str())
        .map(Some)
        .ok_or_else(|| format!("{name} requires a UTF-8 value"))
}

fn option_path(args: &[OsString], name: &str) -> Result<Option<PathBuf>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .map(PathBuf::from)
        .map(Some)
        .ok_or_else(|| format!("{name} requires a path"))
}

fn parse_method(value: &str) -> Result<Method, String> {
    match value {
        "raw" => Ok(Method::Raw),
        "order-0" => Ok(Method::Order0),
        "order-1" => Ok(Method::Order1),
        _ => Err(format!(
            "unknown method {value:?}; expected raw, order-0, or order-1"
        )),
    }
}

fn append_extension(path: &Path, extension: &str) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(".");
    value.push(extension);
    value.into()
}

fn remove_takc_extension(path: &Path) -> PathBuf {
    if path
        .extension()
        .is_some_and(|extension| extension == "takc")
    {
        path.with_extension("")
    } else {
        append_extension(path, "out")
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Err(format!("refusing to overwrite {}", path.display()));
    }
    fs::write(path, bytes).map_err(|error| io_error("write", path, &error))
}

fn io_error(operation: &str, path: &Path, error: &std::io::Error) -> String {
    format!("cannot {operation} {}: {error}", path.display())
}

fn print_help() {
    println!(
        "TAK experimental compressor\n\n\
Usage:\n  tak compress <input> [-o <output>] [--method raw|order-0|order-1]\n  \
tak decompress <input> [-o <output>] [--max-output-bytes <bytes>]\n  \
tak verify <input> [--max-output-bytes <bytes>]\n  \
tak benchmark <input>\n\nThe format is experimental and is not suitable for durable storage."
    );
}
