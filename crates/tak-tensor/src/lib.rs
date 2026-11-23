//! Research-only quantized two-core byte-bigram factorization.
//!
//! Artifacts use `TKRX`, not TAK-C. This crate makes no compression claim.

mod lz;
mod match8;
mod match8_strong;
mod order2_normalized;
mod order2_smoothing;
mod second_order;
mod third_order;
mod third_order_backoff;

pub use lz::{
    LzEncodeStats, LzError, LzPricedResidualProfile, LzStreamError, LzStreamStats,
    decode_lz_artifact, decode_lz_stream, encode_lz_artifact, encode_lz_priced_artifact,
    encode_lz_priced_stream, encode_lz_stream, encode_lz_two_candidate_artifact,
    encode_lz_two_candidate_stream, profile_lz_priced_residual,
};
pub use match8::{
    decode_match8_artifact, decode_match8_stream, encode_match8_artifact, encode_match8_stream,
};
pub use match8_strong::{decode_match8_strong_artifact, encode_match8_strong_artifact};
pub use order2_normalized::{
    StreamArtifactError, StreamArtifactStats, decode_order2_normalized_artifact,
    decode_order2_normalized_stream, encode_order2_normalized_artifact,
    encode_order2_normalized_stream,
};
pub use order2_smoothing::{decode_order2_smoothed_artifact, encode_order2_smoothed_artifact};
pub use second_order::{
    Order2TensorDiagnostics, QuantizedOrder2Model, decode_order2_tensor_artifact,
    encode_order2_tensor_artifact,
};
pub use third_order::{decode_order3_artifact, encode_order3_artifact};
pub use third_order_backoff::{decode_order3_backoff_artifact, encode_order3_backoff_artifact};

use tak_context::{ByteModel, Order2Model};

const ALPHABET: usize = 256;
const MAX_RANK: usize = 8;
const MAX_TOTAL: u64 = 16_384;
const HEADER_LEN: usize = 28;
const MAGIC: &[u8; 4] = b"TKRX";
const ORDER2_MAGIC: &[u8; 4] = b"TKO2";

/// A quantized two-core bigram model.
#[derive(Clone)]
pub struct QuantizedModel {
    rank: usize,
    left: Vec<u16>,
    right: Vec<u16>,
    rows: Vec<Row>,
    context: Option<u8>,
}

/// Cross-entropy diagnostics for one held-out byte sequence.
#[derive(Debug, Clone, Copy)]
pub struct Diagnostics {
    /// Bits per input byte before core quantization and frequency rounding.
    pub full_precision_bits_per_byte: f64,
    /// Bits per input byte after core quantization but before frequency rounding.
    pub quantized_core_bits_per_byte: f64,
    /// Bits per input byte from the exact integer frequencies used by the coder.
    pub integer_frequency_bits_per_byte: f64,
}

#[derive(Clone)]
struct Row {
    frequencies: [u16; ALPHABET],
    total: u32,
}

/// Errors produced by research artifact decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The artifact header or extent is invalid.
    InvalidArtifact,
    /// The rank is zero or exceeds the experiment limit.
    InvalidRank,
    /// The declared output exceeds the caller's limit.
    OutputLimitExceeded,
    /// Arithmetic decoding failed.
    InvalidPayload,
    /// The restored bytes do not match the artifact checksum.
    ChecksumMismatch,
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArtifact => formatter.write_str("invalid TKRX artifact"),
            Self::InvalidRank => formatter.write_str("invalid TKRX model rank"),
            Self::OutputLimitExceeded => formatter.write_str("TKRX output limit exceeded"),
            Self::InvalidPayload => formatter.write_str("invalid TKRX payload"),
            Self::ChecksumMismatch => formatter.write_str("TKRX checksum mismatch"),
        }
    }
}

impl std::error::Error for Error {}

impl QuantizedModel {
    /// Trains a deterministic nonnegative two-core factorization.
    ///
    /// Floating point is used only by the encoder-side training operation. The
    /// returned model contains only integers and is portable to the decoder.
    ///
    /// # Panics
    ///
    /// Panics if `rank` is zero or greater than eight.
    #[must_use]
    pub fn train(training: &[u8], rank: usize, iterations: usize) -> Self {
        assert!((1..=MAX_RANK).contains(&rank));
        let (left, right) = fit(training, rank, iterations);
        let (left, right) = quantize(&left, &right, rank);
        Self::from_cores(rank, left, right).expect("trained cores have valid dimensions")
    }

    /// Trains a model and measures each lossy modeling stage on held-out bytes.
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
    ) -> (Self, Diagnostics) {
        assert!((1..=MAX_RANK).contains(&rank));
        let (float_left, float_right) = fit(training, rank, iterations);
        let full_precision_bits_per_byte =
            float_bits_per_byte(held_out, &float_left, &float_right, rank);
        let (left, right) = quantize(&float_left, &float_right, rank);
        let quantized_core_bits_per_byte = quantized_bits_per_byte(held_out, &left, &right, rank);
        let model =
            Self::from_cores(rank, left, right).expect("trained cores have valid dimensions");
        let integer_frequency_bits_per_byte = integer_bits_per_byte(held_out, &model.rows);
        (
            model,
            Diagnostics {
                full_precision_bits_per_byte,
                quantized_core_bits_per_byte,
                integer_frequency_bits_per_byte,
            },
        )
    }

    /// Returns the tensor-network bond dimension.
    #[must_use]
    pub const fn rank(&self) -> usize {
        self.rank
    }

    /// Returns the serialized core size, excluding the artifact header.
    #[must_use]
    pub const fn model_bytes(&self) -> usize {
        (ALPHABET * 2 * self.rank) * size_of::<u16>()
    }

    fn from_cores(rank: usize, left: Vec<u16>, right: Vec<u16>) -> Result<Self, Error> {
        if !(1..=MAX_RANK).contains(&rank)
            || left.len() != ALPHABET * rank
            || right.len() != ALPHABET * rank
        {
            return Err(Error::InvalidRank);
        }
        let rows = build_rows(&left, &right, rank);
        Ok(Self {
            rank,
            left,
            right,
            rows,
            context: None,
        })
    }
}

impl ByteModel for QuantizedModel {
    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let index = usize::from(symbol);
        let Some(context) = self.context else {
            return (u32::from(symbol), u32::from(symbol) + 1, 256);
        };
        let row = &self.rows[usize::from(context)];
        let low = row.frequencies[..index]
            .iter()
            .map(|&value| u32::from(value))
            .sum();
        (low, low + u32::from(row.frequencies[index]), row.total)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        if self.context.is_none() {
            let symbol = u8::try_from(value).expect("uniform start total is 256");
            return (symbol, value, value + 1, 256);
        }
        let row = &self.rows[usize::from(self.context.expect("context checked"))];
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
        unreachable!("scaled value is below total")
    }

    fn update(&mut self, symbol: u8) {
        self.context = Some(symbol);
    }

    fn total(&self) -> u32 {
        self.context
            .map_or(256, |context| self.rows[usize::from(context)].total)
    }
}

/// Encodes a complete research artifact, including quantized cores.
#[must_use]
pub fn encode_artifact(input: &[u8], model: &QuantizedModel) -> Vec<u8> {
    let payload = tak_entropy::encode(input, model.clone());
    let mut output = Vec::with_capacity(HEADER_LEN + model.model_bytes() + payload.len());
    output.extend_from_slice(MAGIC);
    output.push(0);
    output.push(0);
    let rank = u16::try_from(model.rank).unwrap_or_default();
    output.extend_from_slice(&rank.to_le_bytes());
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    for value in model.left.iter().chain(&model.right) {
        output.extend_from_slice(&value.to_le_bytes());
    }
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN
        || &artifact[..4] != MAGIC
        || artifact[4] != 0
        || artifact[5] != 0
    {
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
    let values = ALPHABET.checked_mul(rank).ok_or(Error::InvalidArtifact)?;
    let model_bytes = values.checked_mul(4).ok_or(Error::InvalidArtifact)?;
    let payload_start = HEADER_LEN
        .checked_add(model_bytes)
        .ok_or(Error::InvalidArtifact)?;
    if artifact.len()
        != payload_start
            .checked_add(payload_len)
            .ok_or(Error::InvalidArtifact)?
    {
        return Err(Error::InvalidArtifact);
    }
    let mut left = Vec::with_capacity(values);
    let mut right = Vec::with_capacity(values);
    for index in 0..values * 2 {
        let offset = HEADER_LEN + index * 2;
        let value = u16::from_le_bytes([artifact[offset], artifact[offset + 1]]);
        if index < values {
            left.push(value);
        } else {
            right.push(value);
        }
    }
    let model = QuantizedModel::from_cores(rank, left, right)?;
    let output_len = usize::try_from(output_len).map_err(|_| Error::InvalidArtifact)?;
    let output = tak_entropy::decode(&artifact[payload_start..], output_len, model)
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

/// Encodes the complete adaptive order-2 research control artifact.
#[must_use]
pub fn encode_order2_artifact(input: &[u8]) -> Vec<u8> {
    let payload = tak_entropy::encode(input, Order2Model::default());
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(ORDER2_MAGIC);
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes the complete adaptive order-2 research control artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_order2_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN
        || &artifact[..4] != ORDER2_MAGIC
        || artifact[4..8] != [0, 0, 0, 0]
    {
        return Err(Error::InvalidArtifact);
    }
    let output_len = read_u64(&artifact[8..16]);
    if output_len > max_output {
        return Err(Error::OutputLimitExceeded);
    }
    let payload_len =
        usize::try_from(read_u64(&artifact[16..24])).map_err(|_| Error::InvalidArtifact)?;
    if artifact.len()
        != HEADER_LEN
            .checked_add(payload_len)
            .ok_or(Error::InvalidArtifact)?
    {
        return Err(Error::InvalidArtifact);
    }
    let expected_crc = u32::from_le_bytes([artifact[24], artifact[25], artifact[26], artifact[27]]);
    let output_len = usize::try_from(output_len).map_err(|_| Error::InvalidArtifact)?;
    let output = tak_entropy::decode(&artifact[HEADER_LEN..], output_len, Order2Model::default())
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

fn bigram_counts(input: &[u8]) -> Vec<f64> {
    let mut counts = vec![0.0; ALPHABET * ALPHABET];
    for pair in input.windows(2) {
        counts[usize::from(pair[0]) * ALPHABET + usize::from(pair[1])] += 1.0;
    }
    counts
}

fn fit(training: &[u8], rank: usize, iterations: usize) -> (Vec<f64>, Vec<f64>) {
    let counts = bigram_counts(training);
    let (mut left, mut right) = initialize(rank);
    for _ in 0..iterations {
        update_right(&counts, &left, &mut right, rank);
        update_left(&counts, &mut left, &right, rank);
    }
    (left, right)
}

fn initialize(rank: usize) -> (Vec<f64>, Vec<f64>) {
    let mut left = vec![0.0; ALPHABET * rank];
    let mut right = vec![0.0; rank * ALPHABET];
    for context in 0..ALPHABET {
        for component in 0..rank {
            let seed = u8::try_from((context * 31 + component * 17) % 29)
                .expect("initialization seed is below 29");
            left[context * rank + component] = 0.5 + f64::from(seed) / 29.0;
        }
    }
    for component in 0..rank {
        for symbol in 0..ALPHABET {
            let seed = u8::try_from((component * 23 + symbol * 19) % 31)
                .expect("initialization seed is below 31");
            right[component * ALPHABET + symbol] = 0.5 + f64::from(seed) / 31.0;
        }
    }
    (left, right)
}

#[allow(clippy::many_single_char_names)]
fn update_right(counts: &[f64], left: &[f64], right: &mut [f64], rank: usize) {
    let mut gram = vec![0.0; rank * rank];
    for a in 0..rank {
        for b in 0..rank {
            gram[a * rank + b] = (0..ALPHABET)
                .map(|i| left[i * rank + a] * left[i * rank + b])
                .sum();
        }
    }
    let old = right.to_vec();
    for a in 0..rank {
        for j in 0..ALPHABET {
            let numerator: f64 = (0..ALPHABET)
                .map(|i| left[i * rank + a] * counts[i * ALPHABET + j])
                .sum();
            let denominator: f64 = (0..rank)
                .map(|b| gram[a * rank + b] * old[b * ALPHABET + j])
                .sum();
            right[a * ALPHABET + j] = old[a * ALPHABET + j] * numerator / denominator.max(1e-12);
        }
    }
}

#[allow(clippy::many_single_char_names)]
fn update_left(counts: &[f64], left: &mut [f64], right: &[f64], rank: usize) {
    let mut gram = vec![0.0; rank * rank];
    for a in 0..rank {
        for b in 0..rank {
            gram[a * rank + b] = (0..ALPHABET)
                .map(|j| right[a * ALPHABET + j] * right[b * ALPHABET + j])
                .sum();
        }
    }
    let old = left.to_vec();
    for i in 0..ALPHABET {
        for a in 0..rank {
            let numerator: f64 = (0..ALPHABET)
                .map(|j| counts[i * ALPHABET + j] * right[a * ALPHABET + j])
                .sum();
            let denominator: f64 = (0..rank)
                .map(|b| old[i * rank + b] * gram[b * rank + a])
                .sum();
            left[i * rank + a] = old[i * rank + a] * numerator / denominator.max(1e-12);
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn quantize(left: &[f64], right: &[f64], rank: usize) -> (Vec<u16>, Vec<u16>) {
    let mut adjusted_left = left.to_vec();
    let mut normalized_right = right.to_vec();
    for component in 0..rank {
        let maximum = (0..ALPHABET)
            .map(|symbol| right[component * ALPHABET + symbol])
            .fold(0.0_f64, f64::max)
            .max(1e-30);
        for context in 0..ALPHABET {
            adjusted_left[context * rank + component] *= maximum;
        }
        for symbol in 0..ALPHABET {
            normalized_right[component * ALPHABET + symbol] /= maximum;
        }
    }
    let left_max = adjusted_left
        .iter()
        .copied()
        .fold(0.0_f64, f64::max)
        .max(1e-30);
    let to_u16 = |value: f64| (value.clamp(0.0, 1.0) * f64::from(u16::MAX)).round() as u16;
    (
        adjusted_left
            .into_iter()
            .map(|value| to_u16(value / left_max))
            .collect(),
        normalized_right.into_iter().map(to_u16).collect(),
    )
}

fn build_rows(left: &[u16], right: &[u16], rank: usize) -> Vec<Row> {
    (0..ALPHABET)
        .map(|context| {
            let scores: Vec<u64> = (0..ALPHABET)
                .map(|symbol| {
                    (0..rank)
                        .map(|component| {
                            u64::from(left[context * rank + component])
                                * u64::from(right[component * ALPHABET + symbol])
                        })
                        .sum()
                })
                .collect();
            let score_total: u64 = scores.iter().sum();
            let mut frequencies = [1_u16; ALPHABET];
            if score_total != 0 {
                for (frequency, score) in frequencies.iter_mut().zip(scores) {
                    let extra = score * (MAX_TOTAL - ALPHABET as u64) / score_total;
                    *frequency += u16::try_from(extra).expect("normalized frequency fits u16");
                }
            }
            let total = frequencies.iter().map(|&value| u32::from(value)).sum();
            Row { frequencies, total }
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn float_bits_per_byte(input: &[u8], left: &[f64], right: &[f64], rank: usize) -> f64 {
    if input.is_empty() {
        return 0.0;
    }
    let counts = bigram_counts(input);
    let mut bits = 8.0;
    for context in 0..ALPHABET {
        let mut scores = [0.0_f64; ALPHABET];
        for symbol in 0..ALPHABET {
            scores[symbol] = (0..rank)
                .map(|component| {
                    left[context * rank + component] * right[component * ALPHABET + symbol]
                })
                .sum();
        }
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
fn quantized_bits_per_byte(input: &[u8], left: &[u16], right: &[u16], rank: usize) -> f64 {
    if input.is_empty() {
        return 0.0;
    }
    let counts = bigram_counts(input);
    let mut bits = 8.0;
    for context in 0..ALPHABET {
        let mut scores = [0_u64; ALPHABET];
        for symbol in 0..ALPHABET {
            scores[symbol] = (0..rank)
                .map(|component| {
                    u64::from(left[context * rank + component])
                        * u64::from(right[component * ALPHABET + symbol])
                })
                .sum();
        }
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
    let counts = bigram_counts(input);
    let mut bits = 8.0;
    for context in 0..ALPHABET {
        let row = &rows[context];
        for symbol in 0..ALPHABET {
            let count = counts[context * ALPHABET + symbol];
            if count != 0.0 {
                let probability = f64::from(row.frequencies[symbol]) / f64::from(row.total);
                bits -= count * probability.log2();
            }
        }
    }
    bits / input.len() as f64
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes.try_into().expect("fixed slice length"))
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::{
        QuantizedModel, decode_artifact, decode_order2_artifact, encode_artifact,
        encode_order2_artifact,
    };

    #[test]
    fn every_experiment_rank_round_trips() {
        let training = b"abracadabra mississippi banana bandana".repeat(100);
        let input = b"banana abracadabra mississippi".repeat(50);
        for rank in [1, 2, 4, 8] {
            let model = QuantizedModel::train(&training, rank, 5);
            let artifact = encode_artifact(&input, &model);
            assert_eq!(
                decode_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            assert_eq!(model.model_bytes(), rank * 1_024);
        }
    }

    #[test]
    fn diagnostics_are_finite_and_match_coder_scale() {
        let training = b"the quick brown fox jumps over the lazy dog".repeat(50);
        let held_out = b"the quick fox jumps over the dog".repeat(20);
        let (model, diagnostics) =
            QuantizedModel::train_with_diagnostics(&training, &held_out, 2, 5);
        assert!(diagnostics.full_precision_bits_per_byte.is_finite());
        assert!(diagnostics.quantized_core_bits_per_byte.is_finite());
        assert!(diagnostics.integer_frequency_bits_per_byte.is_finite());
        let artifact = encode_artifact(&held_out, &model);
        assert_eq!(decode_artifact(&artifact, 10_000).unwrap(), held_out);
    }

    #[test]
    fn rejects_corruption_and_output_excess() {
        let model = QuantizedModel::train(b"abababab", 1, 2);
        let mut artifact = encode_artifact(b"abab", &model);
        assert!(decode_artifact(&artifact, 3).is_err());
        artifact[24] ^= 1;
        assert!(decode_artifact(&artifact, 4).is_err());
    }

    #[test]
    fn every_truncation_is_rejected_without_a_panic() {
        let model = QuantizedModel::train(b"some training bytes", 2, 2);
        let artifact = encode_artifact(b"held out", &model);
        for length in 0..artifact.len() {
            let result = std::panic::catch_unwind(|| decode_artifact(&artifact[..length], 100));
            assert!(result.is_ok());
            assert!(result.unwrap().is_err());
        }
    }

    #[test]
    fn order_2_control_round_trips_and_rejects_truncation() {
        let input = b"two-byte contexts can distinguish these bytes".repeat(100);
        let artifact = encode_order2_artifact(&input);
        assert_eq!(
            decode_order2_artifact(&artifact, input.len() as u64).unwrap(),
            input
        );
        assert!(decode_order2_artifact(&artifact, 10).is_err());
        assert!(decode_order2_artifact(&artifact[..artifact.len() - 1], u64::MAX).is_err());
        let mut corrupt = artifact;
        corrupt[24] ^= 1;
        assert!(decode_order2_artifact(&corrupt, u64::MAX).is_err());
    }
}
