use tak_context::ByteModel;

use crate::{Error, MAX_TOTAL, crc32, read_u64};

const ALPHABET: usize = 256;
const CONTEXTS: usize = ALPHABET * ALPHABET;
const HEADER_LEN: usize = 28;
const MAX_RANK: usize = 8;
const MAGIC: &[u8; 4] = b"TKR3";

#[derive(Clone)]
struct Row {
    frequencies: [u16; ALPHABET],
    total: u32,
}

/// A quantized three-core tensor train conditioned on two preceding bytes.
#[derive(Clone)]
pub struct QuantizedOrder2Model {
    rank: usize,
    first: Vec<u16>,
    middle: Vec<u16>,
    last: Vec<u16>,
    rows: Vec<Row>,
    history: [u8; 2],
    seen: u8,
}

/// Cross-entropy diagnostics for a held-out three-core model.
#[derive(Debug, Clone, Copy)]
pub struct Order2TensorDiagnostics {
    /// Bits per byte before core quantization and frequency rounding.
    pub full_precision_bits_per_byte: f64,
    /// Bits per byte after 16-bit core quantization.
    pub quantized_core_bits_per_byte: f64,
    /// Bits per byte from the exact integer frequencies used by the coder.
    pub integer_frequency_bits_per_byte: f64,
}

impl QuantizedOrder2Model {
    /// Trains a sequential nonnegative three-core factorization.
    ///
    /// Training uses floating point. The returned decoder model contains only
    /// unsigned 16-bit cores and deterministic integer probability tables.
    ///
    /// # Panics
    ///
    /// Panics if `rank` is zero or greater than eight.
    #[must_use]
    pub fn train(training: &[u8], rank: usize, iterations: usize) -> Self {
        assert!((1..=MAX_RANK).contains(&rank));
        let (first, middle, last) = fit(training, rank, iterations);
        let first = quantize(&first);
        let middle = quantize(&middle);
        let last = quantize(&last);
        Self::from_cores(rank, first, middle, last).expect("trained cores have valid dimensions")
    }

    /// Trains a model and attributes each held-out modeling loss.
    ///
    /// # Panics
    ///
    /// Panics if `rank` is zero or greater than eight.
    #[must_use]
    pub fn train_with_diagnostics(
        training: &[u8],
        held_out: &[u8],
        rank: usize,
        iterations: usize,
    ) -> (Self, Order2TensorDiagnostics) {
        assert!((1..=MAX_RANK).contains(&rank));
        let (float_first, float_middle, float_last) = fit(training, rank, iterations);
        let full_precision_bits_per_byte =
            float_bits_per_byte(held_out, &float_first, &float_middle, &float_last, rank);
        let first = quantize(&float_first);
        let middle = quantize(&float_middle);
        let last = quantize(&float_last);
        let quantized_core_bits_per_byte =
            quantized_bits_per_byte(held_out, &first, &middle, &last, rank);
        let model = Self::from_cores(rank, first, middle, last)
            .expect("trained cores have valid dimensions");
        let integer_frequency_bits_per_byte = integer_bits_per_byte(held_out, &model.rows);
        (
            model,
            Order2TensorDiagnostics {
                full_precision_bits_per_byte,
                quantized_core_bits_per_byte,
                integer_frequency_bits_per_byte,
            },
        )
    }

    /// Returns the tensor-train bond dimension.
    #[must_use]
    pub const fn rank(&self) -> usize {
        self.rank
    }

    /// Returns the serialized size of all three cores.
    #[must_use]
    pub const fn model_bytes(&self) -> usize {
        let values = ALPHABET * self.rank * 2 + ALPHABET * self.rank * self.rank;
        values * size_of::<u16>()
    }

    fn from_cores(
        rank: usize,
        first: Vec<u16>,
        middle: Vec<u16>,
        last: Vec<u16>,
    ) -> Result<Self, Error> {
        if !(1..=MAX_RANK).contains(&rank)
            || first.len() != ALPHABET * rank
            || middle.len() != ALPHABET * rank * rank
            || last.len() != ALPHABET * rank
        {
            return Err(Error::InvalidRank);
        }
        let rows = build_rows(&first, &middle, &last, rank);
        Ok(Self {
            rank,
            first,
            middle,
            last,
            rows,
            history: [0; 2],
            seen: 0,
        })
    }
}

impl ByteModel for QuantizedOrder2Model {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        if self.seen < 2 {
            return (u32::from(symbol), u32::from(symbol) + 1, 256);
        }
        let row = &self.rows[context_index(self.history)];
        let index = usize::from(symbol);
        let low = row.frequencies[..index]
            .iter()
            .map(|&frequency| u32::from(frequency))
            .sum();
        (low, low + u32::from(row.frequencies[index]), row.total)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        if self.seen < 2 {
            let symbol = u8::try_from(value).expect("uniform distribution total is 256");
            return (symbol, value, value + 1, 256);
        }
        let row = &self.rows[context_index(self.history)];
        let mut low = 0;
        for (symbol, &frequency) in row.frequencies.iter().enumerate() {
            let high = low + u32::from(frequency);
            if value < high {
                return (
                    u8::try_from(symbol).expect("row has 256 symbols"),
                    low,
                    high,
                    row.total,
                );
            }
            low = high;
        }
        unreachable!("scaled value is below frequency total")
    }

    fn update(&mut self, symbol: u8) {
        match self.seen {
            0 => {
                self.history[1] = symbol;
                self.seen = 1;
            }
            1 => {
                self.history[0] = self.history[1];
                self.history[1] = symbol;
                self.seen = 2;
            }
            _ => {
                self.history[0] = self.history[1];
                self.history[1] = symbol;
            }
        }
    }

    fn total(&self) -> u32 {
        if self.seen < 2 {
            256
        } else {
            self.rows[context_index(self.history)].total
        }
    }
}

/// Encodes a complete `TKR3` research artifact.
#[must_use]
pub fn encode_order2_tensor_artifact(input: &[u8], model: &QuantizedOrder2Model) -> Vec<u8> {
    let payload = tak_entropy::encode(input, model.clone());
    let mut output = Vec::with_capacity(HEADER_LEN + model.model_bytes() + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0, 0]);
    output.extend_from_slice(&u16::try_from(model.rank).unwrap_or_default().to_le_bytes());
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    for value in model.first.iter().chain(&model.middle).chain(&model.last) {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete `TKR3` research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, invalid arithmetic
/// payload, or checksum mismatch.
pub fn decode_order2_tensor_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..6] != [0, 0] {
        return Err(Error::InvalidArtifact);
    }
    let rank = usize::from(u16::from_le_bytes([artifact[6], artifact[7]]));
    if !(1..=MAX_RANK).contains(&rank) {
        return Err(Error::InvalidRank);
    }
    let output_len = read_u64(&artifact[8..16]);
    if output_len > max_output {
        return Err(Error::OutputLimitExceeded);
    }
    let payload_len =
        usize::try_from(read_u64(&artifact[16..24])).map_err(|_| Error::InvalidArtifact)?;
    let expected_crc = u32::from_le_bytes([artifact[24], artifact[25], artifact[26], artifact[27]]);
    let first_values = ALPHABET.checked_mul(rank).ok_or(Error::InvalidArtifact)?;
    let middle_values = first_values
        .checked_mul(rank)
        .ok_or(Error::InvalidArtifact)?;
    let total_values = first_values
        .checked_mul(2)
        .and_then(|value| value.checked_add(middle_values))
        .ok_or(Error::InvalidArtifact)?;
    let payload_start = HEADER_LEN
        .checked_add(total_values.checked_mul(2).ok_or(Error::InvalidArtifact)?)
        .ok_or(Error::InvalidArtifact)?;
    if artifact.len()
        != payload_start
            .checked_add(payload_len)
            .ok_or(Error::InvalidArtifact)?
    {
        return Err(Error::InvalidArtifact);
    }
    let mut values = Vec::with_capacity(total_values);
    for index in 0..total_values {
        let offset = HEADER_LEN + index * 2;
        values.push(u16::from_le_bytes([artifact[offset], artifact[offset + 1]]));
    }
    let middle_end = first_values + middle_values;
    let first = values[..first_values].to_vec();
    let middle = values[first_values..middle_end].to_vec();
    let last = values[middle_end..].to_vec();
    let model = QuantizedOrder2Model::from_cores(rank, first, middle, last)?;
    let output_len = usize::try_from(output_len).map_err(|_| Error::InvalidArtifact)?;
    let output = tak_entropy::decode(&artifact[payload_start..], output_len, model)
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

fn context_index(history: [u8; 2]) -> usize {
    (usize::from(history[0]) << 8) | usize::from(history[1])
}

fn trigram_counts(input: &[u8]) -> Vec<f64> {
    let mut counts = vec![0.0; CONTEXTS * ALPHABET];
    for bytes in input.windows(3) {
        let context = (usize::from(bytes[0]) << 8) | usize::from(bytes[1]);
        counts[context * ALPHABET + usize::from(bytes[2])] += 1.0;
    }
    counts
}

fn fit(training: &[u8], rank: usize, iterations: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let counts = trigram_counts(training);
    let (first, remainder) = nmf(&counts, ALPHABET, CONTEXTS, rank, iterations);
    let (middle, last) = nmf(&remainder, rank * ALPHABET, ALPHABET, rank, iterations);
    (first, middle, last)
}

fn nmf(
    data: &[f64],
    rows: usize,
    columns: usize,
    rank: usize,
    iterations: usize,
) -> (Vec<f64>, Vec<f64>) {
    debug_assert_eq!(data.len(), rows * columns);
    let mut left = initialize(rows, rank, 29, 17);
    let mut right = initialize(rank, columns, 31, 19);
    for _ in 0..iterations {
        let mut gram = vec![0.0; rank * rank];
        for a in 0..rank {
            for b in 0..rank {
                gram[a * rank + b] = (0..rows)
                    .map(|row| left[row * rank + a] * left[row * rank + b])
                    .sum();
            }
        }
        let mut numerator = vec![0.0; rank * columns];
        for row in 0..rows {
            for column in 0..columns {
                let value = data[row * columns + column];
                if value != 0.0 {
                    for component in 0..rank {
                        numerator[component * columns + column] +=
                            left[row * rank + component] * value;
                    }
                }
            }
        }
        let old = right.clone();
        for component in 0..rank {
            for column in 0..columns {
                let denominator: f64 = (0..rank)
                    .map(|other| gram[component * rank + other] * old[other * columns + column])
                    .sum();
                right[component * columns + column] = old[component * columns + column]
                    * numerator[component * columns + column]
                    / denominator.max(1e-30);
            }
        }

        let mut gram = vec![0.0; rank * rank];
        for a in 0..rank {
            for b in 0..rank {
                gram[a * rank + b] = (0..columns)
                    .map(|column| right[a * columns + column] * right[b * columns + column])
                    .sum();
            }
        }
        let mut numerator = vec![0.0; rows * rank];
        for row in 0..rows {
            for column in 0..columns {
                let value = data[row * columns + column];
                if value != 0.0 {
                    for component in 0..rank {
                        numerator[row * rank + component] +=
                            value * right[component * columns + column];
                    }
                }
            }
        }
        let old = left.clone();
        for row in 0..rows {
            for component in 0..rank {
                let denominator: f64 = (0..rank)
                    .map(|other| old[row * rank + other] * gram[other * rank + component])
                    .sum();
                left[row * rank + component] = old[row * rank + component]
                    * numerator[row * rank + component]
                    / denominator.max(1e-30);
            }
        }
    }
    (left, right)
}

fn initialize(rows: usize, columns: usize, modulus: usize, multiplier: usize) -> Vec<f64> {
    let modulus_float = f64::from(u8::try_from(modulus).expect("initialization modulus fits u8"));
    (0..rows * columns)
        .map(|index| {
            let seed = u8::try_from((index * multiplier + 1) % modulus)
                .expect("initialization modulus fits u8");
            0.5 + f64::from(seed) / modulus_float
        })
        .collect()
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize(values: &[f64]) -> Vec<u16> {
    let maximum = values.iter().copied().fold(0.0_f64, f64::max).max(1e-30);
    values
        .iter()
        .map(|value| ((value / maximum).clamp(0.0, 1.0) * f64::from(u16::MAX)).round() as u16)
        .collect()
}

fn build_rows(first: &[u16], middle: &[u16], last: &[u16], rank: usize) -> Vec<Row> {
    let mut rows = Vec::with_capacity(CONTEXTS);
    for first_byte in 0..ALPHABET {
        for second_byte in 0..ALPHABET {
            let mut contracted = vec![0_u64; rank];
            for (right_rank, value) in contracted.iter_mut().enumerate() {
                *value = (0..rank)
                    .map(|left_rank| {
                        u64::from(first[first_byte * rank + left_rank])
                            * u64::from(
                                middle[(left_rank * ALPHABET + second_byte) * rank + right_rank],
                            )
                    })
                    .sum();
            }
            let scores: Vec<u64> = (0..ALPHABET)
                .map(|symbol| {
                    (0..rank)
                        .map(|component| {
                            contracted[component] * u64::from(last[component * ALPHABET + symbol])
                        })
                        .sum()
                })
                .collect();
            let score_total: u64 = scores.iter().sum();
            let mut frequencies = [1_u16; ALPHABET];
            if score_total != 0 {
                for (frequency, score) in frequencies.iter_mut().zip(scores) {
                    let extra = u128::from(score) * u128::from(MAX_TOTAL - ALPHABET as u64)
                        / u128::from(score_total);
                    *frequency += u16::try_from(extra).expect("normalized frequency fits u16");
                }
            }
            let total = frequencies
                .iter()
                .map(|&frequency| u32::from(frequency))
                .sum();
            rows.push(Row { frequencies, total });
        }
    }
    rows
}

fn float_scores(
    context: usize,
    first: &[f64],
    middle: &[f64],
    last: &[f64],
    rank: usize,
) -> [f64; ALPHABET] {
    let first_byte = context / ALPHABET;
    let second_byte = context % ALPHABET;
    let mut contracted = vec![0.0; rank];
    for (right_rank, value) in contracted.iter_mut().enumerate() {
        *value = (0..rank)
            .map(|left_rank| {
                first[first_byte * rank + left_rank]
                    * middle[(left_rank * ALPHABET + second_byte) * rank + right_rank]
            })
            .sum();
    }
    let mut scores = [0.0; ALPHABET];
    for (symbol, score) in scores.iter_mut().enumerate() {
        *score = (0..rank)
            .map(|component| contracted[component] * last[component * ALPHABET + symbol])
            .sum();
    }
    scores
}

fn quantized_scores(
    context: usize,
    first: &[u16],
    middle: &[u16],
    last: &[u16],
    rank: usize,
) -> [u64; ALPHABET] {
    let first_byte = context / ALPHABET;
    let second_byte = context % ALPHABET;
    let mut contracted = vec![0_u64; rank];
    for (right_rank, value) in contracted.iter_mut().enumerate() {
        *value = (0..rank)
            .map(|left_rank| {
                u64::from(first[first_byte * rank + left_rank])
                    * u64::from(middle[(left_rank * ALPHABET + second_byte) * rank + right_rank])
            })
            .sum();
    }
    let mut scores = [0_u64; ALPHABET];
    for (symbol, score) in scores.iter_mut().enumerate() {
        *score = (0..rank)
            .map(|component| contracted[component] * u64::from(last[component * ALPHABET + symbol]))
            .sum();
    }
    scores
}

#[allow(clippy::cast_precision_loss)]
fn float_bits_per_byte(
    input: &[u8],
    first: &[f64],
    middle: &[f64],
    last: &[f64],
    rank: usize,
) -> f64 {
    if input.is_empty() {
        return 0.0;
    }
    let counts = trigram_counts(input);
    let mut bits = f64::from(u8::try_from(input.len().min(2)).expect("at most two")) * 8.0;
    for context in 0..CONTEXTS {
        let scores = float_scores(context, first, middle, last, rank);
        let score_total: f64 = scores.iter().sum();
        for (symbol, score) in scores.into_iter().enumerate() {
            let count = counts[context * ALPHABET + symbol];
            if count != 0.0 {
                let probability = if score_total == 0.0 {
                    1.0 / ALPHABET as f64
                } else {
                    (1.0 + score * (MAX_TOTAL - ALPHABET as u64) as f64 / score_total)
                        / MAX_TOTAL as f64
                };
                bits -= count * probability.log2();
            }
        }
    }
    bits / input.len() as f64
}

#[allow(clippy::cast_precision_loss)]
fn quantized_bits_per_byte(
    input: &[u8],
    first: &[u16],
    middle: &[u16],
    last: &[u16],
    rank: usize,
) -> f64 {
    if input.is_empty() {
        return 0.0;
    }
    let counts = trigram_counts(input);
    let mut bits = f64::from(u8::try_from(input.len().min(2)).expect("at most two")) * 8.0;
    for context in 0..CONTEXTS {
        let scores = quantized_scores(context, first, middle, last, rank);
        let score_total: u64 = scores.iter().sum();
        for (symbol, score) in scores.into_iter().enumerate() {
            let count = counts[context * ALPHABET + symbol];
            if count != 0.0 {
                let probability = if score_total == 0 {
                    1.0 / ALPHABET as f64
                } else {
                    (1.0 + score as f64 * (MAX_TOTAL - ALPHABET as u64) as f64 / score_total as f64)
                        / MAX_TOTAL as f64
                };
                bits -= count * probability.log2();
            }
        }
    }
    bits / input.len() as f64
}

#[allow(clippy::cast_precision_loss)]
fn integer_bits_per_byte(input: &[u8], rows: &[Row]) -> f64 {
    if input.is_empty() {
        return 0.0;
    }
    let counts = trigram_counts(input);
    let mut bits = f64::from(u8::try_from(input.len().min(2)).expect("at most two")) * 8.0;
    for context in 0..CONTEXTS {
        for symbol in 0..ALPHABET {
            let count = counts[context * ALPHABET + symbol];
            if count != 0.0 {
                let probability =
                    f64::from(rows[context].frequencies[symbol]) / f64::from(rows[context].total);
                bits -= count * probability.log2();
            }
        }
    }
    bits / input.len() as f64
}

#[cfg(test)]
mod tests {
    use super::{
        QuantizedOrder2Model, decode_order2_tensor_artifact, encode_order2_tensor_artifact,
    };

    #[test]
    fn all_ranks_round_trip_small_input() {
        let training = b"abc abd abe xbc xbd xbe".repeat(30);
        let input = b"abc xbd abe xbc".repeat(10);
        for rank in [1, 2, 4, 8] {
            let model = QuantizedOrder2Model::train(&training, rank, 2);
            let artifact = encode_order2_tensor_artifact(&input, &model);
            assert_eq!(
                decode_order2_tensor_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn malformed_artifacts_are_rejected() {
        let model = QuantizedOrder2Model::train(b"abc abc abd", 1, 1);
        let artifact = encode_order2_tensor_artifact(b"abc", &model);
        assert!(decode_order2_tensor_artifact(&artifact, 2).is_err());
        assert!(decode_order2_tensor_artifact(&artifact[..artifact.len() - 1], 3).is_err());
    }

    #[test]
    fn diagnostics_are_finite() {
        let training = b"abc abd abe xbc xbd xbe".repeat(10);
        let held_out = b"abc xbd abe".repeat(5);
        let (_, diagnostics) =
            QuantizedOrder2Model::train_with_diagnostics(&training, &held_out, 1, 1);
        assert!(diagnostics.full_precision_bits_per_byte.is_finite());
        assert!(diagnostics.quantized_core_bits_per_byte.is_finite());
        assert!(diagnostics.integer_frequency_bits_per_byte.is_finite());
    }
}
