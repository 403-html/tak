use std::{env, fs, path::PathBuf, process::ExitCode, time::Instant};

use tak_tensor::profile_literal_phase_order2;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-literal-phase-order2: error: {error}");
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
    let profile = profile_literal_phase_order2(input);
    let seconds = start.elapsed().as_secs_f64();
    let saved_bits = profile.control_bits - profile.candidate_bits;
    let input_bytes = u32::try_from(input.len()).map_err(|_| "input exceeds u32")?;
    let saved_bits_per_byte = if input.is_empty() {
        0.0
    } else {
        saved_bits / f64::from(input_bytes)
    };
    println!(
        "input_bytes\tliteral_bytes\tcontrol_bits\tcandidate_bits\tsaved_bits\tsaved_bits_per_byte\tphase0_literals\tphase1_literals\tphase2_literals\tphase3_literals\tphase0_saved_bits\tphase1_saved_bits\tphase2_saved_bits\tphase3_saved_bits\tseconds"
    );
    println!(
        "{}\t{}\t{:.6}\t{:.6}\t{saved_bits:.6}\t{saved_bits_per_byte:.9}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{seconds:.9}",
        profile.input_bytes,
        profile.literal_bytes,
        profile.control_bits,
        profile.candidate_bits,
        profile.literal_bytes_by_phase[0],
        profile.literal_bytes_by_phase[1],
        profile.literal_bytes_by_phase[2],
        profile.literal_bytes_by_phase[3],
        profile.control_bits_by_phase[0] - profile.candidate_bits_by_phase[0],
        profile.control_bits_by_phase[1] - profile.candidate_bits_by_phase[1],
        profile.control_bits_by_phase[2] - profile.candidate_bits_by_phase[2],
        profile.control_bits_by_phase[3] - profile.candidate_bits_by_phase[3],
    );
    Ok(())
}
