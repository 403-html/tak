//! Deterministic byte-probability models used by the experimental codec.
//!
//! Model updates use integers only. Encoder and decoder updates are identical.

use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{BuildHasherDefault, Hasher};
use std::num::NonZeroU16;

const ALPHABET_SIZE: usize = 256;
const MAX_TOTAL: u32 = 16_384;
const START_CONTEXT: usize = 256;
const ORDER2_SINGLE_BASE: usize = 65_536;
const ORDER2_START_CONTEXT: usize = ORDER2_SINGLE_BASE + ALPHABET_SIZE;
const ORDER2_CONTEXT_COUNT: usize = ORDER2_START_CONTEXT + 1;

type ContextMap<V> = HashMap<u32, V, BuildHasherDefault<ContextHasher>>;

#[derive(Default)]
struct ContextHasher(u64);

impl Hasher for ContextHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        // Context maps use u32 keys, so this path is defensive only.
        let mut fallback = DefaultHasher::new();
        fallback.write(bytes);
        self.0 = fallback.finish();
    }

    fn write_u32(&mut self, value: u32) {
        // Odd multiplication is injective before truncation and spreads the
        // sequential byte-context keys across the table without keyed hashing.
        self.0 = u64::from(value).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    }
}

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

/// An adaptive model selected by the two immediately preceding bytes.
///
/// The start state and second byte use dedicated shorter contexts. Two-byte
/// distributions are allocated on first use. The maximum frequency storage is
/// approximately 34 MiB when all contexts occur.
pub struct Order2Model {
    distributions: Vec<Option<Box<Distribution>>>,
    context: usize,
    previous: Option<u8>,
}

#[derive(Clone, Default)]
struct SparseDistribution {
    // Each value is the frequency above the implicit pseudocount of one.
    increments: Vec<(u8, u16)>,
    total: u32,
}

impl SparseDistribution {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let extra_low: u32 = self
            .increments
            .iter()
            .take_while(|(candidate, _)| *candidate < symbol)
            .map(|(_, increment)| u32::from(*increment))
            .sum();
        let increment = self
            .increments
            .binary_search_by_key(&symbol, |(candidate, _)| *candidate)
            .map_or(0, |index| self.increments[index].1);
        let low = u32::from(symbol) + extra_low;
        (low, low + 1 + u32::from(increment), 256 + self.total)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        let total = 256 + self.total;
        let mut low = 0;
        let mut increment_index = 0;
        for symbol in 0_u16..=255 {
            let symbol = u8::try_from(symbol).expect("symbol is at most 255");
            let increment = self
                .increments
                .get(increment_index)
                .filter(|(candidate, _)| *candidate == symbol)
                .map_or(0, |(_, increment)| {
                    increment_index += 1;
                    *increment
                });
            let high = low + 1 + u32::from(increment);
            if value < high {
                return (symbol, low, high, total);
            }
            low = high;
        }
        unreachable!("value is below the sparse distribution total")
    }

    fn update(&mut self, symbol: u8) {
        if 256 + self.total >= MAX_TOTAL {
            for (_, increment) in &mut self.increments {
                *increment /= 2;
            }
            self.increments.retain(|(_, increment)| *increment != 0);
            self.total = self
                .increments
                .iter()
                .map(|(_, increment)| u32::from(*increment))
                .sum();
        }
        match self
            .increments
            .binary_search_by_key(&symbol, |(candidate, _)| *candidate)
        {
            Ok(index) => self.increments[index].1 += 1,
            Err(index) => self.increments.insert(index, (symbol, 1)),
        }
        self.total += 1;
    }
}

/// An adaptive model selected by the three immediately preceding bytes.
///
/// Each observed context stores only symbols that have followed it. The
/// implicit frequency of every other symbol is one. This is mathematically
/// equivalent to a dense adaptive distribution but avoids allocating 512
/// bytes for every observed three-byte context.
#[derive(Default)]
pub struct Order3Model {
    distributions: ContextMap<SparseDistribution>,
    history: [u8; 3],
    seen: u8,
}

impl Order3Model {
    fn context(&self) -> u32 {
        context_key(self.history, self.seen)
    }
}

fn context_key(history: [u8; 3], seen: u8) -> u32 {
    match seen {
        0 => 0,
        1 => 0x0100_0000 | u32::from(history[2]),
        2 => 0x0200_0000 | (u32::from(history[1]) << 8) | u32::from(history[2]),
        _ => {
            0x0300_0000
                | (u32::from(history[0]) << 16)
                | (u32::from(history[1]) << 8)
                | u32::from(history[2])
        }
    }
}

/// An adaptive order-3 model that backs off to order-2 for rare contexts.
///
/// Both component models update after every byte. The order-3 distribution is
/// selected only after its context has accumulated `min_observations` updates.
pub struct Order3BackoffModel {
    order2: Order2Model,
    order3: ContextMap<SparseDistribution>,
    history: [u8; 3],
    seen: u8,
    min_observations: NonZeroU16,
}

impl Order3BackoffModel {
    /// Creates a model with a deterministic context-selection threshold.
    #[must_use]
    pub fn new(min_observations: NonZeroU16) -> Self {
        Self {
            order2: Order2Model::default(),
            order3: ContextMap::default(),
            history: [0; 3],
            seen: 0,
            min_observations,
        }
    }

    fn selected_order3(&self) -> Option<&SparseDistribution> {
        if self.seen < 3 {
            return None;
        }
        self.order3
            .get(&context_key(self.history, self.seen))
            .filter(|row| row.total >= u32::from(self.min_observations.get()))
    }
}

impl ByteModel for Order3BackoffModel {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        self.selected_order3()
            .map_or_else(|| self.order2.range(symbol), |row| row.range(symbol))
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.selected_order3()
            .map_or_else(|| self.order2.symbol(value), |row| row.symbol(value))
    }

    fn update(&mut self, symbol: u8) {
        self.order2.update(symbol);
        if self.seen >= 3 {
            self.order3
                .entry(context_key(self.history, self.seen))
                .or_default()
                .update(symbol);
        }
        self.history.rotate_left(1);
        self.history[2] = symbol;
        self.seen = self.seen.saturating_add(1).min(3);
    }

    fn total(&self) -> u32 {
        self.selected_order3()
            .map_or_else(|| self.order2.total(), |row| 256 + row.total)
    }
}

impl ByteModel for Order3Model {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        self.distributions
            .get(&self.context())
            .map_or((u32::from(symbol), u32::from(symbol) + 1, 256), |row| {
                row.range(symbol)
            })
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.distributions.get(&self.context()).map_or_else(
            || {
                let symbol = u8::try_from(value).expect("uniform distribution total is 256");
                (symbol, value, value + 1, 256)
            },
            |row| row.symbol(value),
        )
    }

    fn update(&mut self, symbol: u8) {
        let context = self.context();
        self.distributions
            .entry(context)
            .or_default()
            .update(symbol);
        self.history.rotate_left(1);
        self.history[2] = symbol;
        self.seen = self.seen.saturating_add(1).min(3);
    }

    fn total(&self) -> u32 {
        self.distributions
            .get(&self.context())
            .map_or(256, |row| 256 + row.total)
    }
}

impl Default for Order2Model {
    fn default() -> Self {
        let mut distributions = Vec::with_capacity(ORDER2_CONTEXT_COUNT);
        distributions.resize_with(ORDER2_CONTEXT_COUNT, || None);
        Self {
            distributions,
            context: ORDER2_START_CONTEXT,
            previous: None,
        }
    }
}

impl ByteModel for Order2Model {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        self.distributions[self.context].as_ref().map_or(
            (u32::from(symbol), u32::from(symbol) + 1, 256),
            |distribution| distribution.range(symbol),
        )
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.distributions[self.context].as_ref().map_or_else(
            || {
                let symbol = u8::try_from(value).expect("uniform distribution total is 256");
                (symbol, value, value + 1, 256)
            },
            |distribution| distribution.symbol(value),
        )
    }

    fn update(&mut self, symbol: u8) {
        self.distributions[self.context]
            .get_or_insert_with(|| Box::new(Distribution::new()))
            .update(symbol);
        self.context = self
            .previous
            .map_or(ORDER2_SINGLE_BASE + usize::from(symbol), |previous| {
                (usize::from(previous) << 8) | usize::from(symbol)
            });
        self.previous = Some(symbol);
    }

    fn total(&self) -> u32 {
        self.distributions[self.context]
            .as_ref()
            .map_or(256, |distribution| distribution.total)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::{
        ByteModel, Distribution, Order0Model, Order1Model, Order2Model, Order3BackoffModel,
        Order3Model, SparseDistribution,
    };

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

    #[test]
    fn order_2_ranges_remain_valid() {
        exercise(Order2Model::default());
    }

    #[test]
    fn order_3_ranges_remain_valid() {
        exercise(Order3Model::default());
    }

    #[test]
    fn sparse_distribution_matches_dense_distribution() {
        let mut sparse = SparseDistribution::default();
        let mut dense = Distribution::new();
        for symbol in (0_u8..=255).cycle().take(40_000) {
            assert_eq!(sparse.range(symbol), dense.range(symbol));
            let (low, _, _) = dense.range(symbol);
            assert_eq!(sparse.symbol(low), dense.symbol(low));
            sparse.update(symbol);
            dense.update(symbol);
        }
    }

    #[test]
    fn order_3_backoff_ranges_remain_valid() {
        let threshold = NonZeroU16::new(4).unwrap();
        exercise(Order3BackoffModel::new(threshold));
    }
}
