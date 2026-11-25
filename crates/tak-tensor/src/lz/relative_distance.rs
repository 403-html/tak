use super::{AdaptiveDistribution, Block, mixture_bits, priced};

/// Distance prediction costs on the frozen RFC 0034 parse.
#[derive(Default)]
pub struct RelativeDistanceProfile {
    /// Count of match blocks.
    pub matches: usize,
    /// Count of matches that reuse the immediately preceding distance.
    pub repeats: usize,
    /// TK32 absolute-distance loss, including bucket remainders.
    pub control_bits: f64,
    /// Signed-distance difference loss, including bucket remainders.
    pub relative_bits: f64,
    /// Loss of the equal-prior mixture.
    pub mixture_bits: f64,
    /// Standalone predictor savings on the exact repeats.
    pub repeat_saved_bits: f64,
}

fn relative_code(previous: usize, current: usize) -> usize {
    match current.cmp(&previous) {
        std::cmp::Ordering::Equal => 1,
        std::cmp::Ordering::Greater => 2 * (current - previous),
        std::cmp::Ordering::Less => 2 * (previous - current) + 1,
    }
}

fn score(model: &mut AdaptiveDistribution, shifted_value: usize) -> f64 {
    let bucket = shifted_value.ilog2();
    let symbol = usize::try_from(bucket).expect("bucket index fits usize");
    let (low, high, total) = model.range(symbol);
    model.update(symbol);
    f64::from(total).log2() - f64::from(high - low).log2() + f64::from(bucket)
}

/// Measure the RFC 0036 distance predictors on the frozen TK32 parse.
#[must_use]
pub fn profile_relative_distances(input: &[u8]) -> RelativeDistanceProfile {
    let mut absolute = AdaptiveDistribution::new(32);
    let mut relative = AdaptiveDistribution::new(32);
    let mut previous = None;
    let mut result = RelativeDistanceProfile::default();
    for block in priced::parse_priced_backoff(input) {
        let Block::Match { distance, .. } = block else {
            continue;
        };
        let control_bits = score(&mut absolute, distance);
        let relative_bits = previous.map_or(control_bits, |last| {
            score(&mut relative, relative_code(last, distance))
        });
        if previous == Some(distance) {
            result.repeats += 1;
            result.repeat_saved_bits += control_bits - relative_bits;
        }
        result.matches += 1;
        result.control_bits += control_bits;
        result.relative_bits += relative_bits;
        previous = Some(distance);
    }
    result.mixture_bits = mixture_bits(result.control_bits, result.relative_bits);
    result
}

#[cfg(test)]
mod tests {
    use super::{AdaptiveDistribution, profile_relative_distances, relative_code, score};

    #[test]
    fn relative_code_distinguishes_zero_and_both_directions() {
        assert_eq!(relative_code(11, 11), 1);
        assert_eq!(relative_code(11, 13), 4);
        assert_eq!(relative_code(13, 11), 5);
    }

    #[test]
    fn score_charges_bucket_remainder() {
        let mut model = AdaptiveDistribution::new(32);
        assert_eq!(score(&mut model, 5).to_bits(), 7.0_f64.to_bits());
    }

    #[test]
    fn no_matches_have_no_distance_cost() {
        let profile = profile_relative_distances(b"ab");
        assert_eq!(profile.matches, 0);
        assert_eq!(profile.mixture_bits.to_bits(), 0.0_f64.to_bits());
    }
}
