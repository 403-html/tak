use std::{env, fs, path::PathBuf, process::ExitCode};

use tak_tensor::profile_lz_priced_residual;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tak-lz-priced-profile: error: {error}");
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
    u32::try_from(input.len()).map_err(|_| "input must not exceed 4294967295 bytes")?;

    let profile = profile_lz_priced_residual(input);
    println!(
        "input_bytes\tliteral_blocks\tmatch_blocks\tliteral_bytes\tmatch_bytes\tdistance_bits\tdistance_remainder_bits\tglobal_remainder_model_bits\tbucket_remainder_model_bits\tshort_match_events\tshort_match_bytes\tshort_match_literal_bits\tshort_match_token_bits\trepeat0_matches\trepeat0_distance_bits\trepeat4_matches\trepeat4_distance_bits\texcludable_literals\texclusion_saved_bits\tliteral_model_bits\ttwo_source_literal_bits\ttwo_source_disagreements\tlatest_source_hits\tolder_source_hits\tglobal_calibrated_literal_bits\tphase_calibrated_literal_bits\tglobal_expert_literal_bits\tphase_expert_literal_bits\tglobal_dynamic_literal_bits\tphase_dynamic_literal_bits\tfixed_phase_literal_bits\tcontext_expert_literal_bits\tinteger_phase_literal_bits\tphase0_events\tphase0_hits\tphase1_events\tphase1_hits\tphase2_events\tphase2_hits\tphase3_events\tphase3_hits"
    );
    println!(
        "{}\t{}\t{}\t{}\t{}\t{:.3}\t{}\t{:.3}\t{:.3}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{:.3}\t{}\t{:.3}\t{}\t{:.3}\t{:.3}\t{:.3}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        profile.input_bytes,
        profile.literal_blocks,
        profile.match_blocks,
        profile.literal_bytes,
        profile.match_bytes,
        profile.distance_bits,
        profile.distance_remainder_bits,
        profile.global_remainder_model_bits,
        profile.bucket_remainder_model_bits,
        profile.short_match_events,
        profile.short_match_bytes,
        profile.short_match_literal_bits,
        profile.short_match_token_bits,
        profile.repeat0_matches,
        profile.repeat0_distance_bits,
        profile.repeat4_matches,
        profile.repeat4_distance_bits,
        profile.excludable_literals,
        profile.exclusion_saved_bits,
        profile.literal_model_bits,
        profile.two_source_literal_bits,
        profile.two_source_disagreements,
        profile.latest_source_hits,
        profile.older_source_hits,
        profile.global_calibrated_literal_bits,
        profile.phase_calibrated_literal_bits,
        profile.global_expert_literal_bits,
        profile.phase_expert_literal_bits,
        profile.global_dynamic_literal_bits,
        profile.phase_dynamic_literal_bits,
        profile.fixed_phase_literal_bits,
        profile.context_expert_literal_bits,
        profile.integer_phase_literal_bits,
        profile.prediction_events_by_phase[0],
        profile.prediction_hits_by_phase[0],
        profile.prediction_events_by_phase[1],
        profile.prediction_hits_by_phase[1],
        profile.prediction_events_by_phase[2],
        profile.prediction_hits_by_phase[2],
        profile.prediction_events_by_phase[3],
        profile.prediction_hits_by_phase[3],
    );
    Ok(())
}
