use std::{env, fs, num::NonZeroU16, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{
    decode_order2_artifact, decode_order2_normalized_artifact, encode_order2_artifact,
    encode_order2_normalized_artifact,
};

const STRENGTHS: [u16; 6] = [1, 4, 16, 64, 256, 1024];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-order2-normalized-gate: error: {error}");
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
    let strengths = value(&args, "--strength")
        .map(|value| {
            value
                .parse::<u16>()
                .ok()
                .filter(|strength| STRENGTHS.contains(strength))
                .map(|strength| vec![strength])
                .ok_or_else(|| "strength must be one of 1, 4, 16, 64, 256, or 1024".to_owned())
        })
        .transpose()?
        .unwrap_or_else(|| STRENGTHS.to_vec());
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
        "strength\tinput_bytes\torder2_bytes\tnormalized_bytes\torder2_encode_seconds\torder2_decode_seconds\tnormalized_encode_seconds\tnormalized_decode_seconds\troundtrip"
    );
    for strength in strengths {
        let strength = NonZeroU16::new(strength).expect("screening strengths are nonzero");
        let start = Instant::now();
        let artifact = encode_order2_normalized_artifact(&input, strength);
        let encode_seconds = start.elapsed().as_secs_f64();
        let start = Instant::now();
        let restored = decode_order2_normalized_artifact(&artifact, input.len() as u64)
            .map_err(|error| error.to_string())?;
        let decode_seconds = start.elapsed().as_secs_f64();
        if restored != input {
            return Err(format!("strength {} round-trip mismatch", strength.get()));
        }
        println!(
            "{}\t{}\t{}\t{}\t{order2_encode_seconds:.6}\t{order2_decode_seconds:.6}\t{encode_seconds:.6}\t{decode_seconds:.6}\ttrue",
            strength.get(),
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
