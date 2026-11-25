use super::{Block, BlockByteModel, HASH_MULTIPLIER, PhaseMatchModel, literal_phase, priced};

const ORDERS: std::ops::RangeInclusive<usize> = 3..=6;
const ROW_BITS: u32 = 13;
const ROWS: usize = 1 << ROW_BITS;
const SCALE: u32 = 1 << 20;
const TOTAL: u32 = SCALE + 256;

#[derive(Clone)]
struct Row {
    key: u64,
    counts: [u16; 256],
    groups: [u16; 16],
    total: u16,
    distinct: u16,
    singletons: u16,
}

impl Row {
    const EMPTY: Self = Self {
        key: 0,
        counts: [0; 256],
        groups: [0; 16],
        total: 0,
        distinct: 0,
        singletons: 0,
    };

    fn update(&mut self, key: u64, byte: u8) {
        if self.key != key {
            *self = Self { key, ..Self::EMPTY };
        }
        if self.total == 4_096 {
            self.total = 0;
            self.distinct = 0;
            self.singletons = 0;
            self.groups.fill(0);
            for (index, count) in self.counts.iter_mut().enumerate() {
                *count /= 2;
                self.groups[index / 16] += *count;
                self.total += *count;
                self.distinct += u16::from(*count != 0);
                self.singletons += u16::from(*count == 1);
            }
        }
        let count = &mut self.counts[usize::from(byte)];
        match *count {
            0 => {
                self.distinct += 1;
                self.singletons += 1;
            }
            1 => self.singletons -= 1,
            _ => {}
        }
        *count += 1;
        self.groups[usize::from(byte) / 16] += 1;
        self.total += 1;
    }

    fn blend_prefix(&self, boundary: usize, prior: u32) -> u32 {
        let group = boundary / 16;
        let prefix: u32 = self.groups[..group]
            .iter()
            .chain(&self.counts[group * 16..boundary])
            .map(|&count| u32::from(count))
            .sum();
        let weight = u32::from(4 + 2 * self.singletons);
        ((u64::from(SCALE) * u64::from(prefix) + u64::from(weight) * u64::from(prior))
            / u64::from(u32::from(self.total) + weight))
        .try_into()
        .expect("cumulative count is bounded by SCALE")
    }

    fn score(&self, byte: u8, [ppm, singleton]: [f64; 2]) -> [f64; 2] {
        let count = f64::from(self.counts[usize::from(byte)]);
        let total = f64::from(self.total);
        let ppm_weight = f64::from(self.distinct);
        let singleton_weight = f64::from(4 + 2 * self.singletons);
        [
            (count + ppm_weight * ppm) / (total + ppm_weight),
            (count + singleton_weight * singleton) / (total + singleton_weight),
        ]
    }
}

pub(super) struct IntegerVariableOrder {
    base: PhaseMatchModel,
    table: Vec<Row>,
    history: u64,
    seen: usize,
    literal: bool,
}

impl IntegerVariableOrder {
    pub(super) fn new() -> Self {
        Self {
            base: PhaseMatchModel::new(),
            table: vec![Row::EMPTY; 4 * ROWS],
            history: 0,
            seen: 0,
            literal: false,
        }
    }

    fn rows(&self) -> impl Iterator<Item = &Row> {
        contexts(self.history, self.seen).filter_map(|(slot, key)| {
            let row = &self.table[slot];
            (row.key == key && row.total != 0).then_some(row)
        })
    }

    fn active(&self) -> bool {
        self.literal && self.base.model.match_prediction().is_none() && self.rows().next().is_some()
    }

    fn boundary(&self, boundary: u16, base: u32, total: u32) -> u32 {
        Self::compose_boundary(boundary, base, total, self.rows())
    }

    fn compose_boundary<'a>(
        boundary: u16,
        base: u32,
        total: u32,
        rows: impl Iterator<Item = &'a Row>,
    ) -> u32 {
        let prior = u32::try_from(u64::from(SCALE) * u64::from(base) / u64::from(total))
            .expect("cumulative count is bounded by SCALE");
        rows.fold(prior, |value, row| {
            row.blend_prefix(usize::from(boundary), value)
        }) + u32::from(boundary)
    }

    fn active_range(&self, byte: u8) -> (u32, u32, u32) {
        let (low, high, total) = self.base.model.baseline_range(byte);
        let index = u16::from(byte);
        (
            self.boundary(index, low, total),
            self.boundary(index + 1, high, total),
            TOTAL,
        )
    }
}

impl BlockByteModel for IntegerVariableOrder {
    fn set_phase(&mut self, phase: Option<usize>) {
        self.literal = phase.is_some();
        self.base.set_phase(phase);
    }

    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        if self.active() {
            self.active_range(symbol)
        } else {
            self.base.range(symbol)
        }
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        if !self.literal || self.base.model.match_prediction().is_some() {
            return self.base.symbol(value);
        }
        let mut rows = [None; 4];
        for (slot, row) in rows.iter_mut().zip(self.rows()) {
            *slot = Some(row);
        }
        if rows[0].is_none() {
            return self.base.symbol(value);
        }
        let (mut start, mut end) = (0_u16, 256_u16);
        let (mut low, mut high) = (0, TOTAL);
        while end - start > 1 {
            let middle = start + (end - start) / 2;
            let byte = u8::try_from(middle).expect("interior byte boundary is below 256");
            let (base, _, total) = self.base.model.baseline_range(byte);
            let boundary = Self::compose_boundary(middle, base, total, rows.into_iter().flatten());
            if value < boundary {
                end = middle;
                high = boundary;
            } else {
                start = middle;
                low = boundary;
            }
        }
        (
            u8::try_from(start).expect("lower byte boundary is below 256"),
            low,
            high,
            TOTAL,
        )
    }

    fn update(&mut self, symbol: u8) {
        self.base.update(symbol);
        for (slot, key) in contexts(self.history, self.seen) {
            self.table[slot].update(key, symbol);
        }
        self.history = (self.history << 8) | u64::from(symbol);
        self.seen = (self.seen + 1).min(6);
    }

    fn total(&self) -> u32 {
        if self.active() {
            TOTAL
        } else {
            self.base.total()
        }
    }
}

fn contexts(history: u64, seen: usize) -> impl Iterator<Item = (usize, u64)> {
    ORDERS
        .take_while(move |&order| order <= seen)
        .map(move |order| {
            let key = history & ((1 << (8 * order)) - 1);
            let hash = key.wrapping_mul(HASH_MULTIPLIER) >> (64 - ROW_BITS);
            (
                (order - 3) * ROWS + usize::try_from(hash).expect("13-bit index fits usize"),
                key,
            )
        })
}

/// Ideal literal coding costs on the RFC 0019 parse.
#[derive(Default)]
pub struct VariableOrderProfile {
    /// Number of literals scored.
    pub literals: usize,
    /// Number of literals with at least one usable longer context.
    pub eligible: usize,
    /// Complete RFC 0019 literal model cost.
    pub control_bits: f64,
    /// Interpolated distinct-count control cost.
    pub ppm_bits: f64,
    /// Singleton-weighted interpolation cost.
    pub singleton_bits: f64,
    /// Allocated continuation row bytes.
    pub table_bytes: usize,
}

/// Scores bounded variable-order models on the frozen RFC 0019 parse.
#[must_use]
pub fn profile_variable_order(input: &[u8]) -> VariableOrderProfile {
    let mut table = vec![Row::EMPTY; 4 * ROWS];
    let mut result = VariableOrderProfile {
        table_bytes: std::mem::size_of_val(table.as_slice()),
        ..VariableOrderProfile::default()
    };
    let mut model = PhaseMatchModel::new();
    let mut history = 0_u64;
    let mut position = 0;
    for block in priced::parse_priced(input) {
        let (length, literal) = match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(position, start);
                (length, true)
            }
            Block::Match { length, .. } => (length, false),
        };
        model.set_phase(None);
        for (offset, &byte) in input[position..position + length].iter().enumerate() {
            let index = position + offset;
            if literal {
                model.set_phase(Some(literal_phase(offset)));
                let (low, high, total) = model.range(byte);
                let base = f64::from(high - low) / f64::from(total);
                let mut probabilities = [base; 2];
                let mut eligible = false;
                if model.model.match_prediction().is_none() {
                    for (slot, key) in contexts(history, index) {
                        let row = &table[slot];
                        if row.key == key && row.total != 0 {
                            eligible = true;
                            probabilities = row.score(byte, probabilities);
                        }
                    }
                }
                result.literals += 1;
                result.eligible += usize::from(eligible);
                result.control_bits -= base.log2();
                result.ppm_bits -= probabilities[0].log2();
                result.singleton_bits -= probabilities[1].log2();
            }
            model.update(byte);
            for (slot, key) in contexts(history, index) {
                table[slot].update(key, byte);
            }
            history = (history << 8) | u64::from(byte);
        }
        position += length;
    }
    debug_assert_eq!(position, input.len());
    result
}

#[cfg(test)]
mod tests {
    use super::{BlockByteModel, IntegerVariableOrder, Row, TOTAL};

    #[test]
    fn integer_ranges_partition_and_invert() {
        let mut model = IntegerVariableOrder::new();
        for &byte in b"abcdefXabcdefYabcdef" {
            model.update(byte);
        }
        model.set_phase(Some(0));
        assert!(model.active());
        assert_eq!(model.rows().count(), 4);
        assert_eq!(model.total(), TOTAL);
        let mut boundary = 0;
        for byte in 0..=u8::MAX {
            let (low, high, total) = model.range(byte);
            assert_eq!(low, boundary);
            assert!(high > low);
            assert_eq!(model.symbol(low), (byte, low, high, total));
            assert_eq!(model.symbol(high - 1), (byte, low, high, total));
            boundary = high;
        }
        assert_eq!(boundary, TOTAL);
    }

    #[test]
    fn row_counts_survive_halving_and_clear_on_replacement() {
        let mut row = Row::EMPTY;
        row.update(1, 0);
        for _ in 0..4_095 {
            row.update(1, 1);
        }
        row.update(1, 2);
        assert_eq!((row.total, row.distinct, row.singletons), (2_048, 2, 1));
        row.update(2, 3);
        assert_eq!((row.total, row.distinct, row.singletons), (1, 1, 1));
        assert_eq!(row.counts[1], 0);
        assert_eq!(row.counts[3], 1);
    }
}
