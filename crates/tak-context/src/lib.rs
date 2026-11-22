//! Deterministic byte-probability models used by the experimental codec.
//!
//! Model updates use integers only. Encoder and decoder updates are identical.

const ALPHABET_SIZE: usize = 256;
const MAX_TOTAL: u32 = 16_384;
const START_CONTEXT: usize = 256;

/// Supplies cumulative integer frequencies to an entropy coder.
pub trait ByteModel {
    /// Returns `(low, high, total)` for `symbol` in the current distribution.
    fn range(&self, symbol: u8) -> (u32, u32, u32);

    /// Finds the symbol whose cumulative range contains `value`.
    ///
    /// `value` must be less than the current total.
    fn symbol(&self, value: u32) -> (u8, u32, u32, u32);

    /// Updates the model after one symbol has been coded.
    fn update(&mut self, symbol: u8);

    /// Returns the current frequency total.
    fn total(&self) -> u32;
}

#[derive(Clone)]
struct Distribution {
    frequencies: [u16; ALPHABET_SIZE],
    total: u32,
}

impl Distribution {
    fn new() -> Self {
        Self {
            frequencies: [1; ALPHABET_SIZE],
            total: 256,
        }
    }

    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let index = usize::from(symbol);
        let low = self.frequencies[..index]
            .iter()
            .map(|&frequency| u32::from(frequency))
            .sum();
        let high = low + u32::from(self.frequencies[index]);
        (low, high, self.total)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        debug_assert!(value < self.total);
        let mut low = 0;
        for (symbol, &frequency) in self.frequencies.iter().enumerate() {
            let high = low + u32::from(frequency);
            if value < high {
                return (
                    u8::try_from(symbol).expect("distribution has 256 symbols"),
                    low,
                    high,
                    self.total,
                );
            }
            low = high;
        }
        unreachable!("value is below the distribution total")
    }

    fn update(&mut self, symbol: u8) {
        if self.total >= MAX_TOTAL {
            self.total = 0;
            for frequency in &mut self.frequencies {
                *frequency = (*frequency).div_ceil(2);
                self.total += u32::from(*frequency);
            }
        }
        self.frequencies[usize::from(symbol)] += 1;
        self.total += 1;
    }
}

/// An adaptive byte model that does not use preceding bytes.
#[derive(Clone)]
pub struct Order0Model {
    distribution: Distribution,
}

impl Default for Order0Model {
    fn default() -> Self {
        Self {
            distribution: Distribution::new(),
        }
    }
}

impl ByteModel for Order0Model {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        self.distribution.range(symbol)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.distribution.symbol(value)
    }

    fn update(&mut self, symbol: u8) {
        self.distribution.update(symbol);
    }

    fn total(&self) -> u32 {
        self.distribution.total
    }
}

/// An adaptive model selected by the immediately preceding byte.
///
/// A separate start context codes the first byte. Its fixed working set is
/// approximately 132 KiB.
#[derive(Clone)]
pub struct Order1Model {
    distributions: Vec<Distribution>,
    context: usize,
}

impl Default for Order1Model {
    fn default() -> Self {
        Self {
            distributions: vec![Distribution::new(); ALPHABET_SIZE + 1],
            context: START_CONTEXT,
        }
    }
}

impl ByteModel for Order1Model {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        self.distributions[self.context].range(symbol)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.distributions[self.context].symbol(value)
    }

    fn update(&mut self, symbol: u8) {
        self.distributions[self.context].update(symbol);
        self.context = usize::from(symbol);
    }

    fn total(&self) -> u32 {
        self.distributions[self.context].total
    }
}

#[cfg(test)]
mod tests {
    use super::{ByteModel, Order0Model, Order1Model};

    fn exercise(mut model: impl ByteModel) {
        for symbol in (0_u8..=255).cycle().take(40_000) {
            let (low, high, total) = model.range(symbol);
            assert!(low < high);
            assert!(high <= total);
            assert_eq!(model.symbol(low), (symbol, low, high, total));
            model.update(symbol);
        }
    }

    #[test]
    fn order_0_ranges_remain_valid_after_rescaling() {
        exercise(Order0Model::default());
    }

    #[test]
    fn order_1_ranges_remain_valid() {
        exercise(Order1Model::default());
    }
}
