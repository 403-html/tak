use super::variable_order::IntegerVariableOrder;
use super::{Block, BlockByteModel, literal_phase, priced};

#[derive(Clone, Copy)]
struct Row {
    counts: [u16; 16],
    total: u16,
}

impl Row {
    const NEW: Self = Self {
        counts: [1; 16],
        total: 16,
    };

    fn bits(self, symbol: usize) -> f64 {
        f64::from(self.total).log2() - f64::from(self.counts[symbol]).log2()
    }

    fn update(&mut self, symbol: usize) {
        if self.total == 4_096 {
            self.total = 0;
            for count in &mut self.counts {
                *count = count.div_ceil(2);
                self.total += *count;
            }
        }
        self.counts[symbol] += 1;
        self.total += 1;
    }
}

fn keys(previous: [u8; 2], byte: u8) -> (usize, usize) {
    let [older, newer] = previous;
    let high = (usize::from(older >> 4) << 4) | usize::from(newer >> 4);
    let low =
        (usize::from(older & 15) << 8) | (usize::from(newer & 15) << 4) | usize::from(byte >> 4);
    (high, low)
}

fn mixture_bits(left: f64, right: f64) -> f64 {
    left.min(right) + 1.0 - (1.0 + 2.0_f64.powf(-(left - right).abs())).log2()
}

/// Literal prediction costs on the frozen RFC 0034 parse.
#[derive(Default)]
pub struct NibbleProfile {
    /// Number of literals scored.
    pub literals: usize,
    /// Complete TK32 literal model loss.
    pub control_bits: f64,
    /// Four-bit-group predictor loss.
    pub nibble_bits: f64,
    /// Equal-prior Bayesian mixture loss.
    pub mixture_bits: f64,
}

/// Measure the RFC 0035 bit-group mixture on the frozen TK32 parse.
#[must_use]
pub fn profile_nibble_mixture(input: &[u8]) -> NibbleProfile {
    let mut model = IntegerVariableOrder::new();
    let mut upper = vec![Row::NEW; 256];
    let mut lower = vec![Row::NEW; 4_096];
    let mut result = NibbleProfile::default();
    let mut previous = [0; 2];
    let mut position = 0;
    for block in priced::parse_priced_backoff(input) {
        let (length, literal) = match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(position, start);
                (length, true)
            }
            Block::Match { length, .. } => (length, false),
        };
        model.set_phase(None);
        for (offset, &byte) in input[position..position + length].iter().enumerate() {
            let (high_key, low_key) = keys(previous, byte);
            let high = usize::from(byte >> 4);
            let low = usize::from(byte & 15);
            if literal {
                model.set_phase(Some(literal_phase(offset)));
                let (start, end, total) = model.range(byte);
                result.control_bits += f64::from(total).log2() - f64::from(end - start).log2();
                result.nibble_bits += upper[high_key].bits(high) + lower[low_key].bits(low);
                result.literals += 1;
            }
            upper[high_key].update(high);
            lower[low_key].update(low);
            model.update(byte);
            previous = [previous[1], byte];
        }
        position += length;
    }
    debug_assert_eq!(position, input.len());
    result.mixture_bits = mixture_bits(result.control_bits, result.nibble_bits);
    result
}

#[cfg(test)]
mod tests {
    use super::{Row, keys, mixture_bits};

    #[test]
    fn context_keys_preserve_nibble_order() {
        assert_eq!(keys([0xa3, 0xc5], 0x72), (0xac, 0x357));
    }

    #[test]
    fn mixture_charges_one_bit_for_a_dominant_expert() {
        assert!(mixture_bits(100.0, 100.0).to_bits() == 100.0_f64.to_bits());
        assert!(mixture_bits(100.0, 2_000.0).to_bits() == 101.0_f64.to_bits());
    }

    #[test]
    fn rescaling_preserves_positive_counts() {
        let mut row = Row::NEW;
        for _ in 0..4_096 {
            row.update(5);
        }
        assert_eq!(row.counts.iter().sum::<u16>(), row.total);
        assert!(row.total < 4_096);
        assert!(row.counts.iter().all(|&count| count > 0));
    }
}
