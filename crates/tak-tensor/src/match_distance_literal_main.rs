use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::{profile_match_distance_literal, profile_short_suffix_backoff};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-match-distance-literal: error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (short_suffix, args) = match args.as_slice() {
        [flag, rest @ ..] if flag == "--short-suffix" => (true, rest),
        rest => (false, rest),
    };
    let (path, tail_bytes) = match args {
        [path] => (PathBuf::from(path), None),
        [path, option, value] if option == "--tail-bytes" => (
            PathBuf::from(path),
            Some(
                value
                    .parse::<usize>()
                    .map_err(|_| "tail-bytes must be a nonnegative integer")?,
            ),
        ),
        _ => return Err("expected [--short-suffix] INPUT [--tail-bytes N]".to_owned()),
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
    let profile = if short_suffix {
        profile_short_suffix_backoff(input)
    } else {
        profile_match_distance_literal(input)
    };
    let seconds = start.elapsed().as_secs_f64();
    let saved_bits = profile.control_bits - profile.candidate_bits;
    let input_bytes = u32::try_from(input.len()).map_err(|_| "input exceeds u32")?;
    let saved_bits_per_byte = if input.is_empty() {
        0.0
    } else {
        saved_bits / f64::from(input_bytes)
    };
    let source_hit_rate = if profile.eligible_literals == 0 {
        0.0
    } else {
        let hits = u32::try_from(profile.source_hits).map_err(|_| "source hits exceed u32")?;
        let eligible =
            u32::try_from(profile.eligible_literals).map_err(|_| "eligible literals exceed u32")?;
        f64::from(hits) / f64::from(eligible)
    };
    println!(
        "input_bytes\tliteral_bytes\teligible_literals\tsource_hits\tsource_hit_rate\tcontrol_bits\tcandidate_bits\tsaved_bits\tsaved_bits_per_byte\tseconds"
    );
    println!(
        "{}\t{}\t{}\t{}\t{source_hit_rate:.9}\t{:.6}\t{:.6}\t{saved_bits:.6}\t{saved_bits_per_byte:.9}\t{seconds:.9}",
        profile.input_bytes,
        profile.literal_bytes,
        profile.eligible_literals,
        profile.source_hits,
        profile.control_bits,
        profile.candidate_bits,
    );
    Ok(())
}
