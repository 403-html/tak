use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::profile_length_conditioned_distance;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-length-distance: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (path, tail_bytes) = match args.as_slice() {
        [path] => (PathBuf::from(path), None),
        [path, option, value] if option == "--tail-bytes" => (
            PathBuf::from(path),
            Some(
                value
                    .parse::<usize>()
                    .map_err(|_| "tail-bytes must be a nonnegative integer")?,
            ),
        ),
        _ => return Err("expected INPUT [--tail-bytes N]".to_owned()),
    };
    let input = fs::read(path).map_err(|error| error.to_string())?;
    let input = if let Some(bytes) = tail_bytes {
        let start = input
            .len()
            .checked_sub(bytes)
            .ok_or("tail-bytes exceeds input length")?;
        &input[start..]
    } else {
        &input
    };

    let start = Instant::now();
    let profile = profile_length_conditioned_distance(input);
    let seconds = start.elapsed().as_secs_f64();
    let saved_bits = profile.control_bits - profile.candidate_bits;
    let input_bytes = u32::try_from(input.len()).map_err(|_| "input exceeds u32")?;
    let saved_bits_per_byte = if input.is_empty() {
        0.0
    } else {
        saved_bits / f64::from(input_bytes)
    };
    println!(
        "input_bytes\tmatch_blocks\tcontrol_bits\tcandidate_bits\tsaved_bits\tsaved_bits_per_byte\tlength8_matches\tlength9_matches\tlength10_matches\tlength11plus_matches\tseconds"
    );
    println!(
        "{}\t{}\t{:.6}\t{:.6}\t{saved_bits:.6}\t{saved_bits_per_byte:.9}\t{}\t{}\t{}\t{}\t{seconds:.9}",
        profile.input_bytes,
        profile.match_blocks,
        profile.control_bits,
        profile.candidate_bits,
        profile.matches_by_class[0],
        profile.matches_by_class[1],
        profile.matches_by_class[2],
        profile.matches_by_class[3],
    );
    Ok(())
}
