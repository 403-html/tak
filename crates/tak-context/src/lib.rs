//! Deterministic byte-probability models used by the experimental codec.
//!
//! Model updates use integers only. Encoder and decoder updates are identical.

use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{BuildHasherDefault, Hasher};
use std::num::{NonZeroU16, NonZeroU32};

const ALPHABET_SIZE: usize = 256;
const GROUP_SIZE: usize = 16;
const UNIT_GROUP_SUM: u16 = 16;
const _: () = assert!(UNIT_GROUP_SUM as usize == GROUP_SIZE);
const GROUP_COUNT: usize = ALPHABET_SIZE / GROUP_SIZE;
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

fn group_sums<const N: usize>(frequencies: &[u16; N]) -> [u16; GROUP_COUNT] {
    let mut groups = [0_u16; GROUP_COUNT];
    for (group, chunk) in groups.iter_mut().zip(frequencies.chunks_exact(GROUP_SIZE)) {
        *group = chunk.iter().sum();
    }
    groups
}

fn prefix_sum(
    frequencies: &[u16; ALPHABET_SIZE],
    groups: &[u16; GROUP_COUNT],
    index: usize,
) -> u32 {
    let group = index / GROUP_SIZE;
    let full: u32 = groups[..group].iter().map(|&sum| u32::from(sum)).sum();
    let partial: u32 = frequencies[group * GROUP_SIZE..index]
        .iter()
        .map(|&frequency| u32::from(frequency))
        .sum();
    full + partial
}

#[derive(Clone)]
struct Distribution {
    frequencies: [u16; ALPHABET_SIZE],
    groups: [u16; GROUP_COUNT],
    total: u32,
}

impl Distribution {
    fn new() -> Self {
        Self {
            frequencies: [1; ALPHABET_SIZE],
            groups: [UNIT_GROUP_SUM; GROUP_COUNT],
            total: 256,
        }
    }

    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let index = usize::from(symbol);
        let low = prefix_sum(&self.frequencies, &self.groups, index);
        let high = low + u32::from(self.frequencies[index]);
        (low, high, self.total)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        debug_assert!(value < self.total);
        let mut low = 0;
        for (group, &sum) in self.groups.iter().enumerate() {
            let group_high = low + u32::from(sum);
            if value >= group_high {
                low = group_high;
                continue;
            }
            let start = group * GROUP_SIZE;
            for (offset, &frequency) in self.frequencies[start..start + GROUP_SIZE]
                .iter()
                .enumerate()
            {
                let high = low + u32::from(frequency);
                if value < high {
                    return (
                        u8::try_from(start + offset).expect("distribution has 256 symbols"),
                        low,
                        high,
                        self.total,
                    );
                }
                low = high;
            }
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
            self.groups = group_sums(&self.frequencies);
        }
        let index = usize::from(symbol);
        self.frequencies[index] += 1;
        self.groups[index / GROUP_SIZE] += 1;
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

/// An adaptive order-2 model with an order-1 hierarchical prior.
///
/// A two-byte context stores only local frequency increments. Its coding
/// distribution adds those increments to a weighted adaptive order-1 parent.
pub struct Order2SmoothedModel {
    parents: Vec<Distribution>,
    locals: Vec<Option<Box<SparseDistribution>>>,
    history: [u8; 2],
    seen: u8,
    parent_weight: NonZeroU16,
}

/// An adaptive order-2 model with a normalized finite-strength parent prior.
pub struct Order2NormalizedModel {
    parents: Vec<Distribution>,
    locals: Vec<Option<Box<LocalIncrements>>>,
    history: [u8; 2],
    seen: u8,
    prior_strength: NonZeroU16,
}

/// A bounded exact-suffix match predictor over normalized order-2.
pub struct Match8Model {
    baseline: Order2NormalizedModel,
    table: Vec<MatchEntry>,
    history: u64,
    seen: u8,
    bonus_divisor: NonZeroU32,
}

const MATCH_TABLE_BITS: u32 = 20;
const MATCH_TABLE_LEN: usize = 1 << MATCH_TABLE_BITS;
const MATCH_HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Clone, Copy, Default)]
struct MatchEntry {
    key: u64,
    continuation: u8,
    occupied: bool,
}

impl Match8Model {
    /// Creates the fixed RFC 0009 model.
    #[must_use]
    pub fn new() -> Self {
        Self::with_bonus_divisor(NonZeroU32::new(4).unwrap_or(NonZeroU32::MIN))
    }

    /// Creates a research model with a specified match-bonus divisor.
    #[must_use]
    pub fn with_bonus_divisor(bonus_divisor: NonZeroU32) -> Self {
        Self {
            baseline: Order2NormalizedModel::new(NonZeroU16::new(4).unwrap_or(NonZeroU16::MIN)),
            table: vec![MatchEntry::default(); MATCH_TABLE_LEN],
            history: 0,
            seen: 0,
            bonus_divisor,
        }
    }

    fn index(key: u64) -> usize {
        let index = key.wrapping_mul(MATCH_HASH_MULTIPLIER) >> (64 - MATCH_TABLE_BITS);
        usize::try_from(index).expect("20-bit index fits usize")
    }

    /// Returns the current exact-match prediction, if its key is present.
    #[must_use]
    pub fn match_prediction(&self) -> Option<u8> {
        if self.seen < 8 {
            return None;
        }
        let entry = self.table[Self::index(self.history)];
        (entry.occupied && entry.key == self.history).then_some(entry.continuation)
    }

    /// Returns the RFC 0006 baseline range before the exact-match bonus.
    #[must_use]
    pub fn baseline_range(&self, symbol: u8) -> (u32, u32, u32) {
        self.baseline.range(symbol)
    }

    /// Decodes one value with the RFC 0006 baseline before the match bonus.
    #[must_use]
    pub fn baseline_symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.baseline.symbol(value)
    }

    /// Returns the RFC 0006 baseline total before the match bonus.
    #[must_use]
    pub fn baseline_total(&self) -> u32 {
        self.baseline.total()
    }
}

impl Default for Match8Model {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteModel for Match8Model {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let (mut low, mut high, total) = self.baseline.range(symbol);
        let Some(prediction) = self.match_prediction() else {
            return (low, high, total);
        };
        let bonus = total / self.bonus_divisor.get();
        match symbol.cmp(&prediction) {
            std::cmp::Ordering::Greater => {
                low += bonus;
                high += bonus;
            }
            std::cmp::Ordering::Equal => high += bonus,
            std::cmp::Ordering::Less => {}
        }
        (low, high, total + bonus)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        let Some(prediction) = self.match_prediction() else {
            return self.baseline.symbol(value);
        };
        let (prediction_low, prediction_high, total) = self.baseline.range(prediction);
        let bonus = total / self.bonus_divisor.get();
        let augmented_prediction_high = prediction_high + bonus;
        if value >= prediction_low && value < augmented_prediction_high {
            return (
                prediction,
                prediction_low,
                augmented_prediction_high,
                total + bonus,
            );
        }
        let baseline_value = if value >= augmented_prediction_high {
            value - bonus
        } else {
            value
        };
        let (symbol, mut low, mut high, _) = self.baseline.symbol(baseline_value);
        if symbol > prediction {
            low += bonus;
            high += bonus;
        }
        (symbol, low, high, total + bonus)
    }

    fn update(&mut self, symbol: u8) {
        self.baseline.update(symbol);
        if self.seen >= 8 {
            self.table[Self::index(self.history)] = MatchEntry {
                key: self.history,
                continuation: symbol,
                occupied: true,
            };
        }
        self.history = (self.history << 8) | u64::from(symbol);
        self.seen = self.seen.saturating_add(1).min(8);
    }

    fn total(&self) -> u32 {
        let total = self.baseline.total();
        total
            + self
                .match_prediction()
                .map_or(0, |_| total / self.bonus_divisor.get())
    }
}

#[derive(Clone)]
struct LocalIncrements {
    frequencies: [u16; ALPHABET_SIZE],
    groups: [u16; GROUP_COUNT],
    total: u32,
}

impl LocalIncrements {
    fn new() -> Self {
        Self {
            frequencies: [0; ALPHABET_SIZE],
            groups: [0; GROUP_COUNT],
            total: 0,
        }
    }

    fn extra_range(&self, symbol: u8) -> (u32, u32) {
        let index = usize::from(symbol);
        let low = prefix_sum(&self.frequencies, &self.groups, index);
        (low, u32::from(self.frequencies[index]))
    }

    fn update(&mut self, symbol: u8) {
        if 256 + self.total >= MAX_TOTAL {
            self.total = 0;
            for frequency in &mut self.frequencies {
                *frequency /= 2;
                self.total += u32::from(*frequency);
            }
            self.groups = group_sums(&self.frequencies);
        }
        let index = usize::from(symbol);
        self.frequencies[index] += 1;
        self.groups[index / GROUP_SIZE] += 1;
        self.total += 1;
    }
}

#[derive(Clone, Default)]
struct SparseDistribution {
    // Each value is the frequency above the implicit pseudocount of one.
    increments: Vec<(u8, u16)>,
    total: u32,
}

impl SparseDistribution {
    fn extra_range(&self, symbol: u8) -> (u32, u32) {
        let extra_low = self
            .increments
            .iter()
            .take_while(|(candidate, _)| *candidate < symbol)
            .map(|(_, increment)| u32::from(*increment))
            .sum();
        let increment = self
            .increments
            .binary_search_by_key(&symbol, |(candidate, _)| *candidate)
            .map_or(0, |index| u32::from(self.increments[index].1));
        (extra_low, increment)
    }

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
        let mut next_symbol = 0_u32;
        for &(symbol, increment) in &self.increments {
            let symbol = u32::from(symbol);
            let gap = symbol - next_symbol;
            if value < low + gap {
                let symbol = next_symbol + value - low;
                return (
                    u8::try_from(symbol).expect("implicit symbol is at most 255"),
                    value,
                    value + 1,
                    total,
                );
            }
            low += gap;
            let high = low + 1 + u32::from(increment);
            if value < high {
                return (
                    u8::try_from(symbol).expect("stored symbol is at most 255"),
                    low,
                    high,
                    total,
                );
            }
            low = high;
            next_symbol = symbol + 1;
        }
        let symbol = next_symbol + value - low;
        (
            u8::try_from(symbol).expect("trailing implicit symbol is at most 255"),
            value,
            value + 1,
            total,
        )
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

impl Order2SmoothedModel {
    /// Creates a model with a fixed nonzero parent weight.
    #[must_use]
    pub fn new(parent_weight: NonZeroU16) -> Self {
        let mut locals = Vec::with_capacity(ORDER2_SINGLE_BASE);
        locals.resize_with(ORDER2_SINGLE_BASE, || None);
        Self {
            parents: vec![Distribution::new(); ALPHABET_SIZE + 1],
            locals,
            history: [0; 2],
            seen: 0,
            parent_weight,
        }
    }

    fn parent(&self) -> &Distribution {
        let index = if self.seen == 0 {
            START_CONTEXT
        } else {
            usize::from(self.history[1])
        };
        &self.parents[index]
    }

    fn local(&self) -> Option<&SparseDistribution> {
        if self.seen < 2 {
            return None;
        }
        let context = (usize::from(self.history[0]) << 8) | usize::from(self.history[1]);
        self.locals[context].as_deref()
    }

    fn combined_total(&self) -> u32 {
        u32::from(self.parent_weight.get()) * self.parent().total
            + self.local().map_or(0, |row| row.total)
    }
}

impl ByteModel for Order2SmoothedModel {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let weight = u32::from(self.parent_weight.get());
        let (parent_low, parent_high, _) = self.parent().range(symbol);
        let (extra_low, increment) = self.local().map_or((0, 0), |row| row.extra_range(symbol));
        let low = weight * parent_low + extra_low;
        (
            low,
            weight * parent_high + extra_low + increment,
            self.combined_total(),
        )
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        debug_assert!(value < self.combined_total());
        let weight = u32::from(self.parent_weight.get());
        let parent = self.parent();
        let local = self.local();
        let mut local_index = 0;
        let mut low = 0;
        for symbol in 0_u8..=255 {
            let increment = local.map_or(0, |row| {
                let Some(&(candidate, increment)) = row.increments.get(local_index) else {
                    return 0;
                };
                if candidate == symbol {
                    local_index += 1;
                    u32::from(increment)
                } else {
                    0
                }
            });
            let frequency = weight * u32::from(parent.frequencies[usize::from(symbol)]) + increment;
            let high = low + frequency;
            if value < high {
                return (symbol, low, high, self.combined_total());
            }
            low = high;
        }
        unreachable!("value is below the combined distribution total")
    }

    fn update(&mut self, symbol: u8) {
        let parent = if self.seen == 0 {
            START_CONTEXT
        } else {
            usize::from(self.history[1])
        };
        self.parents[parent].update(symbol);
        if self.seen >= 2 {
            let context = (usize::from(self.history[0]) << 8) | usize::from(self.history[1]);
            self.locals[context]
                .get_or_insert_with(|| Box::new(SparseDistribution::default()))
                .update(symbol);
        }
        self.history.rotate_left(1);
        self.history[1] = symbol;
        self.seen = self.seen.saturating_add(1).min(2);
    }

    fn total(&self) -> u32 {
        self.combined_total()
    }
}

impl Order2NormalizedModel {
    /// Creates a model with a fixed nonzero prior strength.
    #[must_use]
    pub fn new(prior_strength: NonZeroU16) -> Self {
        let mut locals = Vec::with_capacity(ORDER2_SINGLE_BASE);
        locals.resize_with(ORDER2_SINGLE_BASE, || None);
        Self {
            parents: vec![Distribution::new(); ALPHABET_SIZE + 1],
            locals,
            history: [0; 2],
            seen: 0,
            prior_strength,
        }
    }

    fn parent_index(&self) -> usize {
        if self.seen == 0 {
            START_CONTEXT
        } else {
            usize::from(self.history[1])
        }
    }

    fn local_index(&self) -> Option<usize> {
        (self.seen >= 2).then(|| (usize::from(self.history[0]) << 8) | usize::from(self.history[1]))
    }

    fn local(&self) -> Option<&LocalIncrements> {
        self.local_index()
            .and_then(|context| self.locals[context].as_deref())
    }
}

impl ByteModel for Order2NormalizedModel {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let parent = &self.parents[self.parent_index()];
        if self.seen < 2 {
            return parent.range(symbol);
        }
        let strength = u32::from(self.prior_strength.get());
        let (parent_low, parent_high, parent_total) = parent.range(symbol);
        let (local_low, local_increment) =
            self.local().map_or((0, 0), |row| row.extra_range(symbol));
        let local_total = self.local().map_or(0, |row| row.total);
        let low = local_low * parent_total + strength * parent_low;
        (
            low,
            (local_low + local_increment) * parent_total + strength * parent_high,
            (local_total + strength) * parent_total,
        )
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        let parent = &self.parents[self.parent_index()];
        if self.seen < 2 {
            return parent.symbol(value);
        }
        let strength = u32::from(self.prior_strength.get());
        let Some(local) = self.local() else {
            let (symbol, low, high, parent_total) = parent.symbol(value / strength);
            return (
                symbol,
                strength * low,
                strength * high,
                strength * parent_total,
            );
        };
        let parent_total = parent.total;
        let total = (local.total + strength) * parent_total;
        let mut low = 0;
        for group in 0..GROUP_COUNT {
            let group_high = low
                + u32::from(local.groups[group]) * parent_total
                + strength * u32::from(parent.groups[group]);
            if value >= group_high {
                low = group_high;
                continue;
            }
            for index in group * GROUP_SIZE..(group + 1) * GROUP_SIZE {
                let frequency = u32::from(local.frequencies[index]) * parent_total
                    + strength * u32::from(parent.frequencies[index]);
                let high = low + frequency;
                if value < high {
                    let symbol = u8::try_from(index).expect("distribution has 256 symbols");
                    return (symbol, low, high, total);
                }
                low = high;
            }
        }
        unreachable!("value is below the normalized distribution total")
    }

    fn update(&mut self, symbol: u8) {
        let parent = self.parent_index();
        self.parents[parent].update(symbol);
        if let Some(context) = self.local_index() {
            self.locals[context]
                .get_or_insert_with(|| Box::new(LocalIncrements::new()))
                .update(symbol);
        }
        self.history.rotate_left(1);
        self.history[1] = symbol;
        self.seen = self.seen.saturating_add(1).min(2);
    }

    fn total(&self) -> u32 {
        if self.seen < 2 {
            self.parents[self.parent_index()].total
        } else {
            let parent_total = self.parents[self.parent_index()].total;
            (self.local().map_or(0, |row| row.total) + u32::from(self.prior_strength.get()))
                * parent_total
        }
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
        ByteModel, Distribution, LocalIncrements, Match8Model, Order0Model, Order1Model,
        Order2Model, Order2NormalizedModel, Order2SmoothedModel, Order3BackoffModel, Order3Model,
        SparseDistribution,
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
    fn smoothed_order_2_ranges_remain_valid() {
        for weight in [1, 2, 4, 8, 16, 32, 64] {
            exercise(Order2SmoothedModel::new(NonZeroU16::new(weight).unwrap()));
        }
    }

    #[test]
    fn normalized_order_2_ranges_remain_valid() {
        for strength in [1, 4, 16, 64, 256, 1024] {
            exercise(Order2NormalizedModel::new(
                NonZeroU16::new(strength).unwrap(),
            ));
        }
    }

    #[test]
    fn bounded_match_ranges_remain_valid() {
        exercise(Match8Model::new());
    }

    #[test]
    fn bounded_match_collision_replaces_the_slot() {
        let first_key = 0_u64;
        let index = Match8Model::index(first_key);
        // This is the multiplicative inverse of MATCH_HASH_MULTIPLIER modulo
        // 2^64. Its hash product is one, so its high 20 bits equal key zero.
        let second_key = 0xf1de_83e1_9937_733d;
        assert_eq!(Match8Model::index(second_key), index);
        let mut model = Match8Model::new();
        model.seen = 8;
        model.history = first_key;
        model.update(b'a');
        assert_eq!(model.table[index].key, first_key);
        model.history = second_key;
        model.update(b'b');
        assert_eq!(model.table[index].key, second_key);
        assert_eq!(model.table[index].continuation, b'b');
    }

    #[test]
    fn order_3_ranges_remain_valid() {
        exercise(Order3Model::default());
    }

    #[test]
    fn sparse_distribution_matches_dense_distribution() {
        let mut sparse = SparseDistribution::default();
        let mut dense = Distribution::new();
        for symbol in [0, 0, 7, 7, 7, 200, 255] {
            sparse.update(symbol);
            dense.update(symbol);
        }
        for value in 0..dense.total {
            assert_eq!(sparse.symbol(value), dense.symbol(value));
        }

        let mut sparse = SparseDistribution::default();
        let mut dense = Distribution::new();
        for symbol in (0_u8..=255).cycle().take(40_000) {
            assert_eq!(sparse.range(symbol), dense.range(symbol));
            let (low, _, _) = dense.range(symbol);
            assert_eq!(sparse.symbol(low), dense.symbol(low));
            sparse.update(symbol);
            dense.update(symbol);
        }
        for value in 0..dense.total {
            assert_eq!(sparse.symbol(value), dense.symbol(value));
        }
    }

    #[test]
    fn dense_local_increments_match_sparse_increments() {
        let mut dense = LocalIncrements::new();
        let mut sparse = SparseDistribution::default();
        for symbol in (0_u8..=255).cycle().take(40_000) {
            assert_eq!(dense.extra_range(symbol), sparse.extra_range(symbol));
            assert_eq!(dense.total, sparse.total);
            dense.update(symbol);
            sparse.update(symbol);
        }
    }

    #[test]
    fn order_3_backoff_ranges_remain_valid() {
        let threshold = NonZeroU16::new(4).unwrap();
        exercise(Order3BackoffModel::new(threshold));
    }
}
