use std::cell::Cell;
use std::collections::VecDeque;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::num::NonZeroU16;

use tak_context::{ByteModel, Match8Model, Order2NormalizedModel};

use crate::{HEADER_LEN, crc32, read_u64};

mod priced;

const MAGIC: &[u8; 4] = b"TKLZ";
const PHASE_MAGIC: &[u8; 4] = b"TKP7";
const SHORT_MAGIC: &[u8; 4] = b"TKS3";
const STRUCTURAL_MAGIC: &[u8; 4] = b"TKT9";
const TABLE_BITS: u32 = 20;
const TABLE_LEN: usize = 1 << TABLE_BITS;
const HASH_MULTIPLIER: u64 = 0x9e37_79b9_7f4a_7c15;
const MIN_MATCH: usize = 8;
const MAX_MATCH: usize = 65_535;
const MAX_DISTANCE: usize = 64 * 1024 * 1024;
const MAX_DISTANCE_U64: u64 = 64 * 1024 * 1024;
const INPUT_PAGE_LEN: usize = 64 * 1024;
const INPUT_PAGE_COUNT: usize = 4;
const MAX_TOTAL: u32 = 16_384;
const REPEAT_SLOTS: usize = 4;
const RANGE_MAX: u64 = u32::MAX as u64;
const HALF: u64 = 0x8000_0000;
const QUARTER: u64 = 0x4000_0000;
const THREE_QUARTERS: u64 = 0xc000_0000;

/// Errors produced by the RFC 0014 research artifact decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LzError {
    /// The header, extent, block, or integer field is invalid.
    InvalidArtifact,
    /// The declared output exceeds the caller's limit.
    OutputLimitExceeded,
    /// The arithmetic-coded payload contains an invalid interval.
    InvalidPayload,
    /// The restored bytes do not match the stored checksum.
    ChecksumMismatch,
}

impl std::fmt::Display for LzError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArtifact => formatter.write_str("invalid TKLZ artifact"),
            Self::OutputLimitExceeded => formatter.write_str("TKLZ output limit exceeded"),
            Self::InvalidPayload => formatter.write_str("invalid TKLZ arithmetic payload"),
            Self::ChecksumMismatch => formatter.write_str("TKLZ checksum mismatch"),
        }
    }
}

impl std::error::Error for LzError {}

/// Byte counts produced by bounded RFC 0014 streaming decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LzStreamStats {
    /// Restored bytes written to the output.
    pub output_bytes: u64,
    /// Arithmetic payload bytes consumed from the artifact.
    pub payload_bytes: u64,
}

/// Byte counts produced by bounded RFC 0014 streaming encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LzEncodeStats {
    /// Source bytes read from the input.
    pub input_bytes: u64,
    /// Arithmetic payload bytes written after the artifact header.
    pub payload_bytes: u64,
    /// Complete artifact bytes written to the output.
    pub artifact_bytes: u64,
}

/// Upper-bound diagnostics for mechanisms not used by the RFC 0016 parser.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LzPricedResidualProfile {
    /// Number of input bytes covered by the parse.
    pub input_bytes: usize,
    /// Number of literal blocks.
    pub literal_blocks: usize,
    /// Number of match blocks.
    pub match_blocks: usize,
    /// Number of bytes coded as literals.
    pub literal_bytes: usize,
    /// Number of bytes copied by matches.
    pub match_bytes: usize,
    /// Ideal bits used by the current global literal, match, and end symbols.
    pub global_type_bits: f64,
    /// Ideal literal-or-match bits after removing decoder-known symbols.
    pub structural_type_bits: f64,
    /// Explicit binary literal-or-match decisions in the structural model.
    pub structural_type_decisions: usize,
    /// Match symbols implied by the preceding literal block and output length.
    pub implicit_match_types: usize,
    /// Ideal global bucket and remainder bits for all match lengths.
    pub match_length_bits: f64,
    /// Match-length bits from models split by preceding block type.
    pub state_match_length_bits: f64,
    /// Ideal bucket and remainder bits for all match distances.
    pub distance_bits: f64,
    /// Match-distance bits from bucket models split by preceding block type.
    pub state_distance_bits: f64,
    /// Raw equiprobable remainder bits for all match distances.
    pub distance_remainder_bits: usize,
    /// Ideal remainder cost from one adaptive model per bit position.
    pub global_remainder_model_bits: f64,
    /// Ideal remainder cost from adaptive models keyed by bucket and bit position.
    pub bucket_remainder_model_bits: f64,
    /// Nonoverlapping profitable 3-to-7-byte match opportunities in literal blocks.
    pub short_match_events: usize,
    /// Literal bytes covered by selected short-match opportunities.
    pub short_match_bytes: usize,
    /// RFC 0017 literal bits covered by selected short-match opportunities.
    pub short_match_literal_bits: f64,
    /// Estimated type, length, and distance bits for selected short matches.
    pub short_match_token_bits: f64,
    /// Matches that repeat the immediately preceding distance.
    pub repeat0_matches: usize,
    /// Distance bits used by immediately repeated distances.
    pub repeat0_distance_bits: f64,
    /// Matches that use one of the four most recent distances.
    pub repeat4_matches: usize,
    /// Distance bits used by one of the four most recent distances.
    pub repeat4_distance_bits: f64,
    /// First literals whose preceding match source byte can be excluded.
    pub excludable_literals: usize,
    /// Ideal byte-model bits saved if those source bytes are excluded.
    pub exclusion_saved_bits: f64,
    /// Ideal RFC 0009 model bits for bytes emitted as literals.
    pub literal_model_bits: f64,
    /// Ideal literal bits after adding one fixed bonus for the older source.
    pub two_source_literal_bits: f64,
    /// Literal events where the latest and older exact-suffix sources disagree.
    pub two_source_disagreements: usize,
    /// Disagreement events where the latest source predicts the literal.
    pub latest_source_hits: usize,
    /// Disagreement events where the older source predicts the literal.
    pub older_source_hits: usize,
    /// Ideal literal bits with one global adaptive match-reliability calibrator.
    pub global_calibrated_literal_bits: f64,
    /// Ideal literal bits with adaptive reliability split by literal-run phase.
    pub phase_calibrated_literal_bits: f64,
    /// Exact-suffix prediction events in phase buckets 0, 1-3, 4-15, and 16+.
    pub prediction_events_by_phase: [usize; 4],
    /// Correct exact-suffix predictions in each literal-run phase bucket.
    pub prediction_hits_by_phase: [usize; 4],
    /// Ideal literal bits from one global mixture of fixed match bonuses.
    pub global_expert_literal_bits: f64,
    /// Ideal literal bits from separate fixed-bonus mixtures by run phase.
    pub phase_expert_literal_bits: f64,
    /// Ideal literal bits from one global count-derived integer bonus.
    pub global_dynamic_literal_bits: f64,
    /// Ideal literal bits from count-derived integer bonuses by run phase.
    pub phase_dynamic_literal_bits: f64,
    /// Ideal literal bits from the frozen per-phase fixed bonus mapping.
    pub fixed_phase_literal_bits: f64,
    /// Ideal literal bits from expert mixtures keyed by phase, length, and distance.
    pub context_expert_literal_bits: f64,
    /// Ideal literal bits from the deterministic integer phase mixture.
    pub integer_phase_literal_bits: f64,
}

/// Ideal-bit diagnostic for a candidate with the frozen RFC 0019 literal parse.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LiteralPhaseOrder2Profile {
    /// Number of bytes in the complete input.
    pub input_bytes: usize,
    /// Number of bytes coded as literals by the RFC 0019 parse.
    pub literal_bytes: usize,
    /// Literal counts in RFC 0017's four phase buckets.
    pub literal_bytes_by_phase: [usize; 4],
    /// Ideal bits from the complete RFC 0019 literal model.
    pub control_bits: f64,
    /// Ideal bits from the candidate literal model.
    pub candidate_bits: f64,
    /// Control ideal bits in each literal phase.
    pub control_bits_by_phase: [f64; 4],
    /// Candidate ideal bits in each literal phase.
    pub candidate_bits_by_phase: [f64; 4],
}

/// Ideal-bit diagnostic for RFC 0023's match-distance literal continuation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MatchDistanceLiteralProfile {
    /// Number of bytes in the complete input.
    pub input_bytes: usize,
    /// Number of bytes coded as literals by the RFC 0019 parse.
    pub literal_bytes: usize,
    /// Literal events with an available preceding-distance source byte.
    pub eligible_literals: usize,
    /// Eligible events where the source byte equals the literal.
    pub source_hits: usize,
    /// Ideal bits from the complete RFC 0019 literal model.
    pub control_bits: f64,
    /// Ideal bits after the source-byte expert mixture.
    pub candidate_bits: f64,
}

/// Ideal-bit diagnostic for RFC 0025's length-conditioned distance buckets.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LengthConditionedDistanceProfile {
    /// Number of bytes in the complete input.
    pub input_bytes: usize,
    /// Number of match blocks in the RFC 0019 parse.
    pub match_blocks: usize,
    /// Ideal distance bucket and remainder bits from RFC 0019.
    pub control_bits: f64,
    /// Ideal distance bucket and remainder bits from four length classes.
    pub candidate_bits: f64,
    /// Match counts for lengths 8, 9, 10, and 11 or more.
    pub matches_by_class: [usize; 4],
}

/// Errors produced by bounded RFC 0014 streaming decode.
#[derive(Debug)]
pub enum LzStreamError {
    /// The artifact is invalid.
    Artifact(LzError),
    /// The source exceeds the experimental encoder-position limit.
    InputTooLarge,
    /// Reading or writing failed.
    Io(io::Error),
}

impl std::fmt::Display for LzStreamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(error) => error.fmt(formatter),
            Self::InputTooLarge => formatter.write_str("TKLZ input exceeds u32 positions"),
            Self::Io(error) => write!(formatter, "TKLZ stream I/O failed: {error}"),
        }
    }
}

impl std::error::Error for LzStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Artifact(error) => Some(error),
            Self::InputTooLarge => None,
            Self::Io(error) => Some(error),
        }
    }
}

impl From<io::Error> for LzStreamError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<LzError> for LzStreamError {
    fn from(error: LzError) -> Self {
        Self::Artifact(error)
    }
}

/// Encodes a complete RFC 0014 research artifact.
#[must_use]
pub fn encode_lz_artifact(input: &[u8]) -> Vec<u8> {
    encode_lz_artifact_with_candidates(input, 1)
}

/// Encodes a complete RFC 0015 two-candidate research artifact.
#[must_use]
pub fn encode_lz_two_candidate_artifact(input: &[u8]) -> Vec<u8> {
    encode_lz_artifact_with_candidates(input, 2)
}

/// Encodes a complete RFC 0016 price-based parser research artifact.
///
/// The artifact uses the unchanged RFC 0014 `TKLZ` syntax and decoder.
#[must_use]
pub fn encode_lz_priced_artifact(input: &[u8]) -> Vec<u8> {
    encode_blocks(input, priced::parse_priced(input))
}

/// Encodes an RFC 0017 literal-phase research artifact.
#[must_use]
pub fn encode_lz_phase_artifact(input: &[u8]) -> Vec<u8> {
    encode_blocks_with_model(
        input,
        priced::parse_priced(input),
        *PHASE_MAGIC,
        PhaseMatchModel::new(),
    )
}

/// Encodes an RFC 0018 short-match research artifact.
#[must_use]
pub fn encode_lz_short_artifact(input: &[u8]) -> Vec<u8> {
    let blocks = priced::parse_priced(input);
    encode_short_blocks(input, short_blocks(input, &blocks))
}

/// Encodes an RFC 0019 structural-block-type research artifact.
#[must_use]
pub fn encode_lz_structural_artifact(input: &[u8]) -> Vec<u8> {
    encode_structural_blocks(input, priced::parse_priced(input))
}

/// Encodes an RFC 0028 two-way parser-table candidate in RFC 0019 syntax.
#[must_use]
pub fn encode_lz_associative_artifact(input: &[u8]) -> Vec<u8> {
    encode_structural_blocks(input, priced::parse_priced_associative(input))
}

/// Encodes an RFC 0024 same-distance-coalesced `TKT9` research artifact.
///
/// The decoder and artifact syntax are unchanged from RFC 0019.
#[must_use]
pub fn encode_lz_coalesced_artifact(input: &[u8]) -> Vec<u8> {
    let blocks = coalesce_same_distance(priced::parse_priced(input));
    encode_structural_blocks(input, blocks)
}

/// Encodes an RFC 0021 structurally repriced `TKT9` research artifact.
///
/// The decoder and artifact syntax are unchanged from RFC 0019.
#[must_use]
pub fn encode_lz_structural_repriced_artifact(input: &[u8]) -> Vec<u8> {
    encode_structural_blocks(input, priced::parse_priced_structural(input))
}

/// Encodes an RFC 0016 price-based parser artifact with bounded memory.
///
/// The artifact is byte-identical to [`encode_lz_priced_artifact`].
///
/// # Errors
///
/// Returns an error for I/O failure, a nonempty output, or an input larger than
/// the experimental format's `u32` encoder-position limit.
pub fn encode_lz_priced_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    priced::encode_priced_stream(&mut input, &mut output)
}

/// Encodes an RFC 0017 literal-phase artifact with bounded memory.
///
/// The artifact is byte-identical to [`encode_lz_phase_artifact`].
///
/// # Errors
///
/// Returns an error for I/O failure, a nonempty output, or an input larger than
/// the experimental format's `u32` encoder-position limit.
pub fn encode_lz_phase_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    priced::encode_priced_stream_with_model(
        &mut input,
        &mut output,
        priced::RING_LEN,
        *PHASE_MAGIC,
        PhaseMatchModel::new(),
        false,
    )
}

/// Encodes an RFC 0019 structural-block-type artifact with bounded memory.
///
/// The artifact is byte-identical to [`encode_lz_structural_artifact`].
///
/// # Errors
///
/// Returns an error for I/O failure, a nonempty output, or an input larger than
/// the experimental format's `u32` encoder-position limit.
pub fn encode_lz_structural_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    priced::encode_priced_stream_with_model(
        &mut input,
        &mut output,
        priced::RING_LEN,
        *STRUCTURAL_MAGIC,
        PhaseMatchModel::new(),
        true,
    )
}

/// Profiles repeat-distance and post-match exclusion upper bounds for RFC 0016.
#[must_use]
pub fn profile_lz_priced_residual(input: &[u8]) -> LzPricedResidualProfile {
    let blocks = priced::parse_priced(input);
    let mut result = LzPricedResidualProfile {
        input_bytes: input.len(),
        ..LzPricedResidualProfile::default()
    };
    profile_block_types(&blocks, &mut result);
    profile_match_state(&blocks, &mut result);
    profile_short_matches(input, &blocks, &mut result);
    let mut distances = AdaptiveDistribution::new(32);
    let mut remainder_models = DistanceRemainderModels::new();
    let mut recent = Vec::with_capacity(REPEAT_SLOTS);
    let mut literal_profile = LiteralProfileState {
        model: Match8Model::new(),
        suffix_table: vec![Entry::default(); TABLE_LEN],
        global_reliability: ReliabilityCounts::new(),
        phase_reliability: [ReliabilityCounts::new(); 4],
        global_experts: ExpertMixture::new(),
        phase_experts: [ExpertMixture::new(); 4],
        context_experts: [ExpertMixture::new(); 80],
        integer_phase_experts: [IntegerExpertMixture::new(); 4],
    };
    let mut excluded = None;
    let mut previous_match_distance = None;
    let mut position = 0;

    for block in blocks {
        match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                result.literal_blocks += 1;
                result.literal_bytes += length;
                let length_class = literal_length_class(length);
                let distance_class = match_distance_class(previous_match_distance);
                if let (Some(skip), Some(&byte)) = (excluded, input.get(start)) {
                    if skip != byte {
                        let (_, _, total) = literal_profile.model.range(byte);
                        let (low, high, _) = literal_profile.model.range(skip);
                        result.excludable_literals += 1;
                        result.exclusion_saved_bits +=
                            f64::from(total).log2() - f64::from(total - (high - low)).log2();
                    }
                }
                for (literal_offset, byte_position) in (start..start + length).enumerate() {
                    profile_priced_byte(
                        input,
                        byte_position,
                        Some(LiteralContext {
                            phase: literal_phase(literal_offset),
                            length_class,
                            distance_class,
                        }),
                        &mut literal_profile,
                        &mut result,
                    );
                }
                position += length;
                excluded = None;
            }
            Block::Match { length, distance } => {
                result.match_blocks += 1;
                result.match_bytes += length;
                let bucket_u32 = distance.ilog2();
                let bucket = usize::try_from(bucket_u32).unwrap_or(0);
                let (low, high, total) = distances.range(bucket);
                let distance_bits =
                    f64::from(total).log2() - f64::from(high - low).log2() + f64::from(bucket_u32);
                result.distance_bits += distance_bits;
                remainder_models.profile(distance, bucket, &mut result);
                distances.update(bucket);

                if let Some(slot) = recent.iter().position(|&value| value == distance) {
                    if slot == 0 {
                        result.repeat0_matches += 1;
                        result.repeat0_distance_bits += distance_bits;
                    }
                    result.repeat4_matches += 1;
                    result.repeat4_distance_bits += distance_bits;
                    recent.remove(slot);
                } else if recent.len() == REPEAT_SLOTS {
                    recent.pop();
                }
                recent.insert(0, distance);

                for byte_position in position..position + length {
                    profile_priced_byte(
                        input,
                        byte_position,
                        None,
                        &mut literal_profile,
                        &mut result,
                    );
                }
                position += length;
                previous_match_distance = Some(distance);
                excluded = position
                    .checked_sub(distance)
                    .and_then(|source| input.get(source))
                    .copied();
            }
        }
    }
    debug_assert_eq!(position, input.len());
    result
}

/// Profiles RFC 0022's phase-specific order-2 residual on the RFC 0019 parse.
#[must_use]
pub fn profile_literal_phase_order2(input: &[u8]) -> LiteralPhaseOrder2Profile {
    let blocks = priced::parse_priced(input);
    let strength = NonZeroU16::new(4).unwrap_or(NonZeroU16::MIN);
    let mut phase_models: [Order2NormalizedModel; 4] =
        std::array::from_fn(|_| Order2NormalizedModel::new(strength));
    let mut mixtures = [TwoExpertMixture::new(); 4];
    let mut control = PhaseMatchModel::new();
    let mut result = LiteralPhaseOrder2Profile {
        input_bytes: input.len(),
        ..LiteralPhaseOrder2Profile::default()
    };
    let mut position = 0;

    for block in blocks {
        match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                for (offset, &byte) in input[start..start + length].iter().enumerate() {
                    let phase = literal_phase(offset);
                    control.set_phase(Some(phase));
                    let (control_low, control_high, control_total) = control.range(byte);
                    let control_probability =
                        f64::from(control_high - control_low) / f64::from(control_total);
                    let (phase_low, phase_high, phase_total) = phase_models[phase].range(byte);
                    let phase_probability =
                        f64::from(phase_high - phase_low) / f64::from(phase_total);
                    let control_bits = -control_probability.log2();
                    let candidate_bits =
                        mixtures[phase].code([control_probability, phase_probability]);

                    result.literal_bytes += 1;
                    result.literal_bytes_by_phase[phase] += 1;
                    result.control_bits += control_bits;
                    result.candidate_bits += candidate_bits;
                    result.control_bits_by_phase[phase] += control_bits;
                    result.candidate_bits_by_phase[phase] += candidate_bits;

                    control.update(byte);
                    for (index, model) in phase_models.iter_mut().enumerate() {
                        if index == phase {
                            model.update(byte);
                        } else {
                            model.observe(byte);
                        }
                    }
                    position += 1;
                }
            }
            Block::Match { length, .. } => {
                control.set_phase(None);
                for &byte in &input[position..position + length] {
                    control.update(byte);
                    for model in &mut phase_models {
                        model.observe(byte);
                    }
                }
                position += length;
            }
        }
    }
    debug_assert_eq!(position, input.len());
    result
}

/// Profiles RFC 0029's literal-only probability counts.
#[must_use]
pub fn profile_literal_only_counts(input: &[u8]) -> LiteralPhaseOrder2Profile {
    let mut control = PhaseMatchModel::new();
    let mut candidate = PhaseMatchModel::new();
    let mut result = LiteralPhaseOrder2Profile {
        input_bytes: input.len(),
        ..LiteralPhaseOrder2Profile::default()
    };
    let mut position = 0;
    for block in priced::parse_priced(input) {
        match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                for (offset, &byte) in input[start..start + length].iter().enumerate() {
                    let phase = literal_phase(offset);
                    control.set_phase(Some(phase));
                    candidate.set_phase(Some(phase));
                    let bits =
                        |(low, high, total)| f64::from(total).log2() - f64::from(high - low).log2();
                    result.control_bits_by_phase[phase] += bits(control.range(byte));
                    result.candidate_bits_by_phase[phase] += bits(candidate.range(byte));
                    result.literal_bytes_by_phase[phase] += 1;
                    control.update(byte);
                    candidate.update(byte);
                }
                position += length;
            }
            Block::Match { length, .. } => {
                control.set_phase(None);
                candidate.set_phase(None);
                for &byte in &input[position..position + length] {
                    control.update(byte);
                    candidate.model.observe(byte);
                }
                position += length;
            }
        }
    }
    debug_assert_eq!(position, input.len());
    result.literal_bytes = result.literal_bytes_by_phase.iter().sum();
    result.control_bits = result.control_bits_by_phase.iter().sum();
    result.candidate_bits = result.candidate_bits_by_phase.iter().sum();
    result
}

/// Profiles RFC 0023's match-distance source on the RFC 0019 parse.
#[must_use]
pub fn profile_match_distance_literal(input: &[u8]) -> MatchDistanceLiteralProfile {
    let blocks = priced::parse_priced(input);
    let mut control = PhaseMatchModel::new();
    let mut mixture = ExpertMixture::new();
    let mut result = MatchDistanceLiteralProfile {
        input_bytes: input.len(),
        ..MatchDistanceLiteralProfile::default()
    };
    let mut previous_distance = None;
    let mut position = 0;

    for block in blocks {
        match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                for (offset, &byte) in input[start..start + length].iter().enumerate() {
                    control.set_phase(Some(literal_phase(offset)));
                    let (low, high, total) = control.range(byte);
                    let width = high - low;
                    let control_bits = f64::from(total).log2() - f64::from(width).log2();
                    let candidate_bits = previous_distance
                        .and_then(|distance| position.checked_sub(distance))
                        .map_or(control_bits, |source| {
                            let hit = input[source] == byte;
                            result.eligible_literals += 1;
                            result.source_hits += usize::from(hit);
                            mixture.code(expert_probabilities(width, total, hit))
                        });
                    result.literal_bytes += 1;
                    result.control_bits += control_bits;
                    result.candidate_bits += candidate_bits;
                    control.update(byte);
                    position += 1;
                }
            }
            Block::Match { length, distance } => {
                control.set_phase(None);
                for &byte in &input[position..position + length] {
                    control.update(byte);
                }
                position += length;
                previous_distance = Some(distance);
            }
        }
    }
    debug_assert_eq!(position, input.len());
    result
}

/// Profiles RFC 0025's current-length-conditioned distance distributions.
#[must_use]
pub fn profile_length_conditioned_distance(input: &[u8]) -> LengthConditionedDistanceProfile {
    let blocks = priced::parse_priced(input);
    let mut global = AdaptiveDistribution::new(32);
    let mut by_length = std::array::from_fn::<_, 4, _>(|_| AdaptiveDistribution::new(32));
    let mut result = LengthConditionedDistanceProfile {
        input_bytes: input.len(),
        ..LengthConditionedDistanceProfile::default()
    };

    for block in blocks {
        let Block::Match { length, distance } = block else {
            continue;
        };
        let class = (length - MIN_MATCH).min(3);
        let bucket_u32 = distance.ilog2();
        let bucket = usize::try_from(bucket_u32).unwrap_or(0);
        let remainder_bits = f64::from(bucket_u32);
        let (low, high, total) = global.range(bucket);
        result.control_bits +=
            f64::from(total).log2() - f64::from(high - low).log2() + remainder_bits;
        global.update(bucket);

        let model = &mut by_length[class];
        let (low, high, total) = model.range(bucket);
        result.candidate_bits +=
            f64::from(total).log2() - f64::from(high - low).log2() + remainder_bits;
        model.update(bucket);
        result.match_blocks += 1;
        result.matches_by_class[class] += 1;
    }
    result
}

fn profile_block_types(blocks: &[Block], result: &mut LzPricedResidualProfile) {
    let mut global = AdaptiveDistribution::new(3);
    let mut structural = AdaptiveDistribution::new(2);
    let mut previous_was_literal = false;

    for &block in blocks {
        let symbol = usize::from(matches!(block, Block::Match { .. }));
        let (low, high, total) = global.range(symbol);
        result.global_type_bits += f64::from(total).log2() - f64::from(high - low).log2();
        global.update(symbol);

        if previous_was_literal {
            debug_assert!(matches!(block, Block::Match { .. }));
            result.implicit_match_types += 1;
        } else {
            let (low, high, total) = structural.range(symbol);
            result.structural_type_bits += f64::from(total).log2() - f64::from(high - low).log2();
            result.structural_type_decisions += 1;
            structural.update(symbol);
        }
        previous_was_literal = matches!(block, Block::Literal { .. });
    }

    let (low, high, total) = global.range(2);
    result.global_type_bits += f64::from(total).log2() - f64::from(high - low).log2();
}

fn profile_match_state(blocks: &[Block], result: &mut LzPricedResidualProfile) {
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut state_match_lengths: [AdaptiveDistribution; 2] =
        std::array::from_fn(|_| AdaptiveDistribution::new(32));
    let mut state_distances: [AdaptiveDistribution; 2] =
        std::array::from_fn(|_| AdaptiveDistribution::new(32));
    let mut previous_was_literal = false;

    for &block in blocks {
        let Block::Match { length, distance } = block else {
            previous_was_literal = true;
            continue;
        };
        let state = usize::from(previous_was_literal);
        let length_value = length - MIN_MATCH;
        let length_bucket_u32 = (length_value + 1).ilog2();
        let length_bucket = usize::try_from(length_bucket_u32).unwrap_or(0);
        let (low, high, total) = match_lengths.range(length_bucket);
        result.match_length_bits +=
            f64::from(total).log2() - f64::from(high - low).log2() + f64::from(length_bucket_u32);
        match_lengths.update(length_bucket);
        let contextual = &mut state_match_lengths[state];
        let (low, high, total) = contextual.range(length_bucket);
        result.state_match_length_bits +=
            f64::from(total).log2() - f64::from(high - low).log2() + f64::from(length_bucket_u32);
        contextual.update(length_bucket);

        let bucket_u32 = distance.ilog2();
        let bucket = usize::try_from(bucket_u32).unwrap_or(0);
        let contextual = &mut state_distances[state];
        let (low, high, total) = contextual.range(bucket);
        result.state_distance_bits +=
            f64::from(total).log2() - f64::from(high - low).log2() + f64::from(bucket_u32);
        contextual.update(bucket);
        previous_was_literal = false;
    }
}

struct DistanceRemainderModels {
    global: Vec<AdaptiveDistribution>,
    by_bucket: Vec<AdaptiveDistribution>,
}

#[derive(Clone, Copy, Default)]
struct ShortEntry {
    key: u32,
    position: u32,
    occupied: bool,
}

#[derive(Clone, Copy)]
struct ShortSelection {
    position: usize,
    length: usize,
    distance: usize,
    literal_bits: f64,
    token_bits: f64,
}

fn profile_short_matches(input: &[u8], blocks: &[Block], result: &mut LzPricedResidualProfile) {
    for selection in short_selections(input, blocks) {
        result.short_match_events += 1;
        result.short_match_bytes += selection.length;
        result.short_match_literal_bits += selection.literal_bits;
        result.short_match_token_bits += selection.token_bits;
    }
}

fn short_selections(input: &[u8], blocks: &[Block]) -> Vec<ShortSelection> {
    let literal_costs = rfc0017_literal_costs(input, blocks);
    let mut table = vec![ShortEntry::default(); TABLE_LEN];
    let mut selections = Vec::new();
    let mut position = 0;
    for &block in blocks {
        let literal_end = match block {
            Block::Literal { length, .. } | Block::Match { length, .. } => position + length,
        };
        let mut selected_until = position;
        while position < literal_end {
            let is_literal = matches!(block, Block::Literal { .. });
            if is_literal && position >= selected_until {
                if let Some((length, distance)) = short_match(input, position, literal_end, &table)
                {
                    let literal_bits: f64 = literal_costs[position..position + length].iter().sum();
                    let token_bits = 5.0 + f64::from(distance.ilog2());
                    if literal_bits > token_bits {
                        selections.push(ShortSelection {
                            position,
                            length,
                            distance,
                            literal_bits,
                            token_bits,
                        });
                        selected_until = position + length;
                    }
                }
            }
            insert_short(input, position, &mut table);
            position += 1;
        }
    }
    selections
}

fn short_blocks(input: &[u8], blocks: &[Block]) -> Vec<ShortBlock> {
    let selections = short_selections(input, blocks);
    let mut selection_index = 0;
    let mut result = Vec::with_capacity(blocks.len() + selections.len() * 2);
    for &block in blocks {
        match block {
            Block::Literal { start, length } => {
                let end = start + length;
                let mut cursor = start;
                while let Some(selection) = selections.get(selection_index) {
                    if selection.position >= end {
                        break;
                    }
                    if cursor < selection.position {
                        result.push(ShortBlock::Literal {
                            start: cursor,
                            length: selection.position - cursor,
                        });
                    }
                    result.push(ShortBlock::ShortMatch {
                        length: selection.length,
                        distance: selection.distance,
                    });
                    cursor = selection.position + selection.length;
                    selection_index += 1;
                }
                if cursor < end {
                    result.push(ShortBlock::Literal {
                        start: cursor,
                        length: end - cursor,
                    });
                }
            }
            Block::Match { length, distance } => {
                result.push(ShortBlock::LongMatch { length, distance });
            }
        }
    }
    debug_assert_eq!(selection_index, selections.len());
    result
}

fn rfc0017_literal_costs(input: &[u8], blocks: &[Block]) -> Vec<f64> {
    let mut costs = vec![0.0; input.len()];
    let mut model = PhaseMatchModel::new();
    let mut position = 0;
    for &block in blocks {
        match block {
            Block::Literal { length, .. } => {
                for offset in 0..length {
                    model.set_phase(Some(literal_phase(offset)));
                    let byte = input[position];
                    let (low, high, total) = model.range(byte);
                    costs[position] = f64::from(total).log2() - f64::from(high - low).log2();
                    model.update(byte);
                    position += 1;
                }
            }
            Block::Match { length, .. } => {
                model.set_phase(None);
                for &byte in &input[position..position + length] {
                    model.update(byte);
                }
                position += length;
            }
        }
    }
    costs
}

fn short_match(
    input: &[u8],
    position: usize,
    literal_end: usize,
    table: &[ShortEntry],
) -> Option<(usize, usize)> {
    let key = short_key(input, position)?;
    let entry = table[short_table_index(key)];
    if !entry.occupied || entry.key != key {
        return None;
    }
    let source = usize::try_from(entry.position).ok()?;
    let distance = position.checked_sub(source)?;
    if distance == 0 || distance > MAX_DISTANCE {
        return None;
    }
    let limit = 7.min(literal_end - position);
    if limit < 3 {
        return None;
    }
    let mut length = 3;
    while length < limit && input[source + length] == input[position + length] {
        length += 1;
    }
    Some((length, distance))
}

fn insert_short(input: &[u8], position: usize, table: &mut [ShortEntry]) {
    let Some(key) = short_key(input, position) else {
        return;
    };
    table[short_table_index(key)] = ShortEntry {
        key,
        position: u32::try_from(position).expect("input is bounded to u32"),
        occupied: true,
    };
}

fn short_key(input: &[u8], position: usize) -> Option<u32> {
    let bytes = input.get(position..position + 3)?;
    Some((u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]))
}

fn short_table_index(key: u32) -> usize {
    usize::try_from(key.wrapping_mul(0x9e37_79b9) >> (32 - TABLE_BITS))
        .expect("20-bit index fits usize")
}

impl DistanceRemainderModels {
    fn new() -> Self {
        Self {
            global: (0..32).map(|_| AdaptiveDistribution::new(2)).collect(),
            by_bucket: (0..32 * 32).map(|_| AdaptiveDistribution::new(2)).collect(),
        }
    }

    fn profile(&mut self, distance: usize, bucket: usize, result: &mut LzPricedResidualProfile) {
        result.distance_remainder_bits += bucket;
        let remainder = distance - (1_usize << bucket);
        for (bit_position, shift) in (0..bucket).zip((0..bucket).rev()) {
            let bit = (remainder >> shift) & 1;
            let global = &mut self.global[bit_position];
            let (low, high, total) = global.range(bit);
            result.global_remainder_model_bits +=
                f64::from(total).log2() - f64::from(high - low).log2();
            global.update(bit);

            let by_bucket = &mut self.by_bucket[bucket * 32 + bit_position];
            let (low, high, total) = by_bucket.range(bit);
            result.bucket_remainder_model_bits +=
                f64::from(total).log2() - f64::from(high - low).log2();
            by_bucket.update(bit);
        }
    }
}

struct LiteralProfileState {
    model: Match8Model,
    suffix_table: Vec<Entry>,
    global_reliability: ReliabilityCounts,
    phase_reliability: [ReliabilityCounts; 4],
    global_experts: ExpertMixture,
    phase_experts: [ExpertMixture; 4],
    context_experts: [ExpertMixture; 80],
    integer_phase_experts: [IntegerExpertMixture; 4],
}

#[derive(Clone, Copy)]
struct ReliabilityCounts {
    hits: u32,
    misses: u32,
}

impl ReliabilityCounts {
    const fn new() -> Self {
        Self { hits: 1, misses: 1 }
    }

    fn prediction_probability(self) -> f64 {
        f64::from(self.hits) / f64::from(self.hits + self.misses)
    }

    fn update(&mut self, hit: bool) {
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
    }

    fn bonus(self, baseline_total: u32, prediction_width: u32) -> u32 {
        let observations = u64::from(self.hits + self.misses);
        let wanted = u64::from(self.hits) * u64::from(baseline_total);
        let baseline = observations * u64::from(prediction_width);
        if wanted <= baseline {
            return 0;
        }
        let bonus = (wanted - baseline) / u64::from(self.misses);
        u32::try_from(bonus.min(u64::from(baseline_total))).unwrap_or(baseline_total)
    }
}

const EXPERTS: usize = 6;
const EXPERT_DIVISORS: [u32; EXPERTS] = [1, 2, 4, 8, 16, u32::MAX];
const PHASE_EXPERT_INDEX: [usize; 4] = [5, 3, 2, 2];

#[derive(Clone, Copy)]
struct LiteralContext {
    phase: usize,
    length_class: usize,
    distance_class: usize,
}

impl LiteralContext {
    const fn index(self) -> usize {
        (self.phase * 4 + self.length_class) * 5 + self.distance_class
    }
}

#[derive(Clone, Copy)]
struct ExpertMixture {
    log_weights: [f64; EXPERTS],
}

impl ExpertMixture {
    const fn new() -> Self {
        Self {
            log_weights: [0.0; EXPERTS],
        }
    }

    fn code(&mut self, probabilities: [f64; EXPERTS]) -> f64 {
        let maximum = self
            .log_weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let weights = self.log_weights.map(|weight| (weight - maximum).exp());
        let weight_total: f64 = weights.iter().sum();
        let probability = weights
            .iter()
            .zip(probabilities)
            .map(|(weight, expert)| weight * expert)
            .sum::<f64>()
            / weight_total;
        for (weight, expert) in self.log_weights.iter_mut().zip(probabilities) {
            *weight += expert.ln();
        }
        let updated_maximum = self
            .log_weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        for weight in &mut self.log_weights {
            *weight -= updated_maximum;
        }
        -probability.log2()
    }
}

#[derive(Clone, Copy)]
struct TwoExpertMixture {
    log_weights: [f64; 2],
}

impl TwoExpertMixture {
    const fn new() -> Self {
        Self {
            log_weights: [0.0; 2],
        }
    }

    fn code(&mut self, probabilities: [f64; 2]) -> f64 {
        let maximum = self
            .log_weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let weights = self.log_weights.map(|weight| (weight - maximum).exp());
        let probability = weights
            .iter()
            .zip(probabilities)
            .map(|(weight, expert)| weight * expert)
            .sum::<f64>()
            / weights.iter().sum::<f64>();
        for (weight, expert) in self.log_weights.iter_mut().zip(probabilities) {
            *weight += expert.ln();
        }
        let updated_maximum = self
            .log_weights
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        for weight in &mut self.log_weights {
            *weight -= updated_maximum;
        }
        -probability.log2()
    }
}

#[derive(Clone, Copy)]
struct IntegerExpertMixture {
    weights: [u32; EXPERTS],
}

impl IntegerExpertMixture {
    const SCALE: u128 = 1 << 32;
    const WEIGHT_TOTAL: u128 = 65_535;

    const fn new() -> Self {
        Self {
            weights: [1; EXPERTS],
        }
    }

    fn bonus(self, total: u32) -> u32 {
        let bonuses = EXPERT_DIVISORS
            .map(|divisor| (divisor != u32::MAX).then(|| total / divisor).unwrap_or(0));
        let scaled_weights = std::array::from_fn::<u128, EXPERTS, _>(|index| {
            u128::from(self.weights[index]) * Self::SCALE / u128::from(total + bonuses[index])
        });
        let denominator: u128 = scaled_weights.iter().sum();
        let numerator: u128 = scaled_weights
            .iter()
            .zip(bonuses)
            .map(|(weight, bonus)| weight * u128::from(bonus))
            .sum();
        u32::try_from(numerator / denominator)
            .unwrap_or(total)
            .min(total)
    }

    fn update(&mut self, actual_width: u32, total: u32, hit: bool) {
        let bonuses = EXPERT_DIVISORS
            .map(|divisor| (divisor != u32::MAX).then(|| total / divisor).unwrap_or(0));
        let scores = std::array::from_fn::<u128, EXPERTS, _>(|index| {
            let width = actual_width + u32::from(hit) * bonuses[index];
            u128::from(self.weights[index]) * u128::from(width) * Self::SCALE
                / u128::from(total + bonuses[index])
        });
        let score_total: u128 = scores.iter().sum();
        self.weights = scores.map(|score| {
            let normalized = score * Self::WEIGHT_TOTAL / score_total;
            u32::try_from(normalized.max(1)).unwrap_or(u32::MAX)
        });
    }

    fn code(&mut self, actual_width: u32, total: u32, hit: bool) -> f64 {
        let bits = bonus_bits(actual_width, total, self.bonus(total), hit);
        self.update(actual_width, total, hit);
        bits
    }
}

const fn literal_phase(offset: usize) -> usize {
    match offset {
        0 => 0,
        1..=3 => 1,
        4..=15 => 2,
        _ => 3,
    }
}

const fn literal_length_class(length: usize) -> usize {
    match length {
        0 | 1 => 0,
        2..=3 => 1,
        4..=15 => 2,
        _ => 3,
    }
}

const fn match_distance_class(distance: Option<usize>) -> usize {
    match distance {
        None => 0,
        Some(0..=256) => 1,
        Some(257..=65_536) => 2,
        Some(65_537..=1_048_576) => 3,
        Some(_) => 4,
    }
}

fn profile_priced_byte(
    input: &[u8],
    position: usize,
    literal_context: Option<LiteralContext>,
    state: &mut LiteralProfileState,
    result: &mut LzPricedResidualProfile,
) {
    let byte = input[position];
    let (low, high, total) = state.model.range(byte);
    let baseline_bits = f64::from(total).log2() - f64::from(high - low).log2();
    let mut candidate_bits = baseline_bits;
    let mut global_calibrated_bits = baseline_bits;
    let mut phase_calibrated_bits = baseline_bits;
    let mut global_expert_bits = baseline_bits;
    let mut phase_expert_bits = baseline_bits;
    let mut global_dynamic_bits = baseline_bits;
    let mut phase_dynamic_bits = baseline_bits;
    let mut fixed_phase_bits = baseline_bits;
    let mut context_expert_bits = baseline_bits;
    let mut integer_phase_bits = baseline_bits;

    if let (Some(context), Some(prediction)) = (literal_context, state.model.match_prediction()) {
        let phase = context.phase;
        let (prediction_low, prediction_high, _) = state.model.range(prediction);
        let prediction_width = prediction_high - prediction_low;
        let hit = prediction == byte;
        global_calibrated_bits = reliability_bits(
            state.global_reliability.prediction_probability(),
            hit,
            high - low,
            total - prediction_width,
        );
        phase_calibrated_bits = reliability_bits(
            state.phase_reliability[phase].prediction_probability(),
            hit,
            high - low,
            total - prediction_width,
        );
        result.prediction_events_by_phase[phase] += 1;
        result.prediction_hits_by_phase[phase] += usize::from(hit);
        let (baseline_actual_low, baseline_actual_high, baseline_total) =
            state.model.baseline_range(byte);
        let baseline_actual_width = baseline_actual_high - baseline_actual_low;
        let (baseline_prediction_low, baseline_prediction_high, _) =
            state.model.baseline_range(prediction);
        let baseline_prediction_width = baseline_prediction_high - baseline_prediction_low;
        let global_bonus = state
            .global_reliability
            .bonus(baseline_total, baseline_prediction_width);
        let phase_bonus =
            state.phase_reliability[phase].bonus(baseline_total, baseline_prediction_width);
        global_dynamic_bits = bonus_bits(baseline_actual_width, baseline_total, global_bonus, hit);
        phase_dynamic_bits = bonus_bits(baseline_actual_width, baseline_total, phase_bonus, hit);
        let probabilities = expert_probabilities(baseline_actual_width, baseline_total, hit);
        global_expert_bits = state.global_experts.code(probabilities);
        phase_expert_bits = state.phase_experts[phase].code(probabilities);
        fixed_phase_bits = -probabilities[PHASE_EXPERT_INDEX[phase]].log2();
        context_expert_bits = state.context_experts[context.index()].code(probabilities);
        integer_phase_bits =
            state.integer_phase_experts[phase].code(baseline_actual_width, baseline_total, hit);
        state.global_reliability.update(hit);
        state.phase_reliability[phase].update(hit);
    }

    if let Some(suffix_start) = position.checked_sub(8) {
        if let Some(key) = key_at(input, suffix_start) {
            let entry = state.suffix_table[table_index(key)];
            if entry.occupied && entry.key == key {
                let latest = input[usize::try_from(entry.position).unwrap_or(0) + 8];
                debug_assert_eq!(state.model.match_prediction(), Some(latest));
                if entry.has_previous {
                    let older = input[usize::try_from(entry.previous_position).unwrap_or(0) + 8];
                    if literal_context.is_some() && older != latest {
                        result.two_source_disagreements += 1;
                        result.latest_source_hits += usize::from(latest == byte);
                        result.older_source_hits += usize::from(older == byte);
                        let bonus = total / 5;
                        if bonus != 0 {
                            let width = high - low + u32::from(older == byte) * bonus;
                            candidate_bits =
                                f64::from(total + bonus).log2() - f64::from(width).log2();
                        }
                    }
                }
            }
            state.model.update(byte);
            insert(input, suffix_start, &mut state.suffix_table);
        } else {
            state.model.update(byte);
        }
    } else {
        state.model.update(byte);
    }

    if literal_context.is_some() {
        result.literal_model_bits += baseline_bits;
        result.two_source_literal_bits += candidate_bits;
        result.global_calibrated_literal_bits += global_calibrated_bits;
        result.phase_calibrated_literal_bits += phase_calibrated_bits;
        result.global_expert_literal_bits += global_expert_bits;
        result.phase_expert_literal_bits += phase_expert_bits;
        result.global_dynamic_literal_bits += global_dynamic_bits;
        result.phase_dynamic_literal_bits += phase_dynamic_bits;
        result.fixed_phase_literal_bits += fixed_phase_bits;
        result.context_expert_literal_bits += context_expert_bits;
        result.integer_phase_literal_bits += integer_phase_bits;
    }
}

fn expert_probabilities(actual_width: u32, total: u32, hit: bool) -> [f64; EXPERTS] {
    EXPERT_DIVISORS.map(|divisor| {
        let bonus = (divisor != u32::MAX).then(|| total / divisor).unwrap_or(0);
        let width = actual_width + u32::from(hit) * bonus;
        f64::from(width) / f64::from(total + bonus)
    })
}

fn bonus_bits(actual_width: u32, total: u32, bonus: u32, hit: bool) -> f64 {
    let width = actual_width + u32::from(hit) * bonus;
    f64::from(total + bonus).log2() - f64::from(width).log2()
}

fn reliability_bits(
    prediction_probability: f64,
    hit: bool,
    actual_width: u32,
    nonprediction_total: u32,
) -> f64 {
    if hit {
        -prediction_probability.log2()
    } else {
        -(1.0 - prediction_probability).log2() + f64::from(nonprediction_total).log2()
            - f64::from(actual_width).log2()
    }
}

trait BlockByteModel {
    fn set_phase(&mut self, phase: Option<usize>);
    fn range(&self, symbol: u8) -> (u32, u32, u32);
    fn symbol(&self, value: u32) -> (u8, u32, u32, u32);
    fn update(&mut self, symbol: u8);
    fn total(&self) -> u32;
}

struct StandardMatchModel(Match8Model);

impl StandardMatchModel {
    fn new() -> Self {
        Self(Match8Model::new())
    }
}

impl BlockByteModel for StandardMatchModel {
    fn set_phase(&mut self, _phase: Option<usize>) {}

    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        self.0.range(symbol)
    }

    fn symbol(&self, value: u32) -> (u8, u32, u32, u32) {
        self.0.symbol(value)
    }

    fn update(&mut self, symbol: u8) {
        self.0.update(symbol);
    }

    fn total(&self) -> u32 {
        self.0.total()
    }
}

struct PhaseMatchModel {
    model: Match8Model,
    mixtures: [IntegerExpertMixture; 4],
    phase: Option<usize>,
    decoded_prediction_bonus: Cell<Option<(u8, u32)>>,
    pending_mixture_update: Cell<Option<(usize, u32, u32, bool)>>,
}

impl PhaseMatchModel {
    fn new() -> Self {
        Self {
            model: Match8Model::new(),
            mixtures: [IntegerExpertMixture::new(); 4],
            phase: None,
            decoded_prediction_bonus: Cell::new(None),
            pending_mixture_update: Cell::new(None),
        }
    }

    fn prediction_bonus(&self) -> Option<(u8, u32)> {
        let phase = self.phase?;
        let prediction = self.model.match_prediction()?;
        Some((
            prediction,
            self.mixtures[phase].bonus(self.model.baseline_total()),
        ))
    }
}

impl BlockByteModel for PhaseMatchModel {
    fn set_phase(&mut self, phase: Option<usize>) {
        self.phase = phase;
        self.decoded_prediction_bonus.set(None);
        self.pending_mixture_update.set(None);
    }

    fn range(&self, symbol: u8) -> (u32, u32, u32) {
        let (mut low, mut high, total) = self.model.baseline_range(symbol);
        let Some((prediction, bonus)) = self.prediction_bonus() else {
            return (low, high, total);
        };
        if let Some(phase) = self.phase {
            self.pending_mixture_update
                .set(Some((phase, high - low, total, symbol == prediction)));
        }
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
        let Some((prediction, bonus)) = self
            .decoded_prediction_bonus
            .take()
            .or_else(|| self.prediction_bonus())
        else {
            return self.model.baseline_symbol(value);
        };
        let (prediction_low, prediction_high, total) = self.model.baseline_range(prediction);
        let augmented_high = prediction_high + bonus;
        if value >= prediction_low && value < augmented_high {
            if let Some(phase) = self.phase {
                self.pending_mixture_update.set(Some((
                    phase,
                    prediction_high - prediction_low,
                    total,
                    true,
                )));
            }
            return (prediction, prediction_low, augmented_high, total + bonus);
        }
        let baseline_value = if value >= augmented_high {
            value - bonus
        } else {
            value
        };
        let (symbol, mut low, mut high, _) = self.model.baseline_symbol(baseline_value);
        if symbol > prediction {
            low += bonus;
            high += bonus;
        }
        if let Some(phase) = self.phase {
            self.pending_mixture_update
                .set(Some((phase, high - low, total, false)));
        }
        (symbol, low, high, total + bonus)
    }

    fn update(&mut self, symbol: u8) {
        self.decoded_prediction_bonus.set(None);
        if let Some((phase, width, total, hit)) = self.pending_mixture_update.take() {
            self.mixtures[phase].update(width, total, hit);
        }
        ByteModel::update(&mut self.model, symbol);
    }

    fn total(&self) -> u32 {
        let prediction_bonus = self.prediction_bonus();
        self.decoded_prediction_bonus.set(prediction_bonus);
        prediction_bonus.map_or_else(
            || self.model.baseline_total(),
            |(_, bonus)| self.model.baseline_total() + bonus,
        )
    }
}

fn encode_lz_artifact_with_candidates(input: &[u8], candidates: usize) -> Vec<u8> {
    encode_blocks(input, parse(input, candidates))
}

fn encode_blocks(input: &[u8], blocks: Vec<Block>) -> Vec<u8> {
    encode_blocks_with_model(input, blocks, *MAGIC, StandardMatchModel::new())
}

fn encode_blocks_with_model(
    input: &[u8],
    blocks: Vec<Block>,
    magic: [u8; 4],
    mut model: impl BlockByteModel,
) -> Vec<u8> {
    let mut encoder = RangeEncoder::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut position = 0;

    for block in blocks {
        match block {
            Block::Literal { start, length } => {
                debug_assert_eq!(start, position);
                encode_adaptive(&mut encoder, &mut types, 0);
                encode_bucket(&mut encoder, &mut literal_lengths, length - 1);
                for (offset, &byte) in input[start..start + length].iter().enumerate() {
                    model.set_phase(Some(literal_phase(offset)));
                    let (low, high, total) = model.range(byte);
                    encoder.encode(low, high, total);
                    model.update(byte);
                }
                position += length;
            }
            Block::Match { length, distance } => {
                model.set_phase(None);
                encode_adaptive(&mut encoder, &mut types, 1);
                encode_bucket(&mut encoder, &mut match_lengths, length - MIN_MATCH);
                encode_bucket(&mut encoder, &mut distances, distance - 1);
                for &byte in &input[position..position + length] {
                    model.update(byte);
                }
                position += length;
            }
        }
    }
    debug_assert_eq!(position, input.len());
    encode_adaptive(&mut encoder, &mut types, 2);
    let payload = encoder.finish();

    let mut artifact = Vec::with_capacity(HEADER_LEN + payload.len());
    artifact.extend_from_slice(&magic);
    artifact.extend_from_slice(&[0; 4]);
    artifact.extend_from_slice(&(input.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&crc32(input).to_le_bytes());
    artifact.extend_from_slice(&payload);
    artifact
}

fn encode_short_blocks(input: &[u8], blocks: Vec<ShortBlock>) -> Vec<u8> {
    let mut encoder = RangeEncoder::new();
    let mut model = PhaseMatchModel::new();
    let mut types = AdaptiveDistribution::new(4);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut short_lengths = AdaptiveDistribution::new(5);
    let mut distances = AdaptiveDistribution::new(32);
    let mut position = 0;

    for block in blocks {
        match block {
            ShortBlock::Literal { start, length } => {
                debug_assert_eq!(start, position);
                encode_adaptive(&mut encoder, &mut types, 0);
                encode_bucket(&mut encoder, &mut literal_lengths, length - 1);
                for (offset, &byte) in input[start..start + length].iter().enumerate() {
                    model.set_phase(Some(literal_phase(offset)));
                    let (low, high, total) = model.range(byte);
                    encoder.encode(low, high, total);
                    model.update(byte);
                }
                position += length;
            }
            ShortBlock::LongMatch { length, distance } => {
                model.set_phase(None);
                encode_adaptive(&mut encoder, &mut types, 1);
                encode_bucket(&mut encoder, &mut match_lengths, length - MIN_MATCH);
                encode_bucket(&mut encoder, &mut distances, distance - 1);
                for &byte in &input[position..position + length] {
                    model.update(byte);
                }
                position += length;
            }
            ShortBlock::ShortMatch { length, distance } => {
                model.set_phase(None);
                encode_adaptive(&mut encoder, &mut types, 2);
                encode_adaptive(&mut encoder, &mut short_lengths, length - 3);
                encode_bucket(&mut encoder, &mut distances, distance - 1);
                for &byte in &input[position..position + length] {
                    model.update(byte);
                }
                position += length;
            }
        }
    }
    debug_assert_eq!(position, input.len());
    encode_adaptive(&mut encoder, &mut types, 3);
    let payload = encoder.finish();

    let mut artifact = Vec::with_capacity(HEADER_LEN + payload.len());
    artifact.extend_from_slice(SHORT_MAGIC);
    artifact.extend_from_slice(&[0; 4]);
    artifact.extend_from_slice(&(input.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&crc32(input).to_le_bytes());
    artifact.extend_from_slice(&payload);
    artifact
}

fn encode_structural_blocks(input: &[u8], blocks: Vec<Block>) -> Vec<u8> {
    let mut encoder = RangeEncoder::new();
    let mut model = PhaseMatchModel::new();
    let mut types = AdaptiveDistribution::new(2);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut position = 0;
    let mut previous_was_literal = false;

    for block in blocks {
        match block {
            Block::Literal { start, length } => {
                debug_assert!(!previous_was_literal);
                debug_assert_eq!(start, position);
                encode_adaptive(&mut encoder, &mut types, 0);
                encode_bucket(&mut encoder, &mut literal_lengths, length - 1);
                for (offset, &byte) in input[start..start + length].iter().enumerate() {
                    model.set_phase(Some(literal_phase(offset)));
                    let (low, high, total) = model.range(byte);
                    encoder.encode(low, high, total);
                    model.update(byte);
                }
                position += length;
                previous_was_literal = true;
            }
            Block::Match { length, distance } => {
                model.set_phase(None);
                if !previous_was_literal {
                    encode_adaptive(&mut encoder, &mut types, 1);
                }
                encode_bucket(&mut encoder, &mut match_lengths, length - MIN_MATCH);
                encode_bucket(&mut encoder, &mut distances, distance - 1);
                for &byte in &input[position..position + length] {
                    model.update(byte);
                }
                position += length;
                previous_was_literal = false;
            }
        }
    }
    debug_assert_eq!(position, input.len());
    let payload = encoder.finish();

    let mut artifact = Vec::with_capacity(HEADER_LEN + payload.len());
    artifact.extend_from_slice(STRUCTURAL_MAGIC);
    artifact.extend_from_slice(&[0; 4]);
    artifact.extend_from_slice(&(input.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    artifact.extend_from_slice(&crc32(input).to_le_bytes());
    artifact.extend_from_slice(&payload);
    artifact
}

fn stream_crc(input: &mut (impl Read + Seek), input_len: u64) -> Result<Crc32, LzStreamError> {
    input.seek(SeekFrom::Start(0))?;
    let mut crc = Crc32::new();
    let mut buffer = vec![0_u8; INPUT_PAGE_LEN];
    let mut remaining = input_len;
    while remaining != 0 {
        let wanted = usize::try_from(remaining.min(INPUT_PAGE_LEN as u64))
            .map_err(|_| LzError::InvalidArtifact)?;
        input.read_exact(&mut buffer[..wanted])?;
        crc.update_slice(&buffer[..wanted]);
        remaining -= u64::try_from(wanted).map_err(|_| LzError::InvalidArtifact)?;
    }
    Ok(crc)
}

/// Encodes an RFC 0014 artifact without retaining the complete input, block
/// list, or arithmetic payload.
///
/// The input and output must be seekable. The encoder uses four 64 KiB input
/// pages in addition to the fixed match and probability-model tables. It
/// patches the artifact header after the payload length is known.
///
/// # Errors
///
/// Returns an error for I/O failure or an input larger than the experimental
/// format's `u32` encoder-position limit.
pub fn encode_lz_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    encode_lz_stream_with_candidates(&mut input, &mut output, 1)
}

/// Encodes an RFC 0015 two-candidate artifact with bounded memory.
///
/// # Errors
///
/// Returns an error for I/O failure, a nonempty output, or an input larger than
/// the experimental format's `u32` encoder-position limit.
pub fn encode_lz_two_candidate_stream(
    mut input: impl Read + Seek,
    mut output: impl Write + Seek,
) -> Result<LzEncodeStats, LzStreamError> {
    encode_lz_stream_with_candidates(&mut input, &mut output, 2)
}

fn encode_lz_stream_with_candidates(
    input: &mut (impl Read + Seek),
    output: &mut (impl Write + Seek),
    candidates: usize,
) -> Result<LzEncodeStats, LzStreamError> {
    let input_len = input.seek(SeekFrom::End(0))?;
    if input_len > u64::from(u32::MAX) {
        return Err(LzStreamError::InputTooLarge);
    }
    let crc = stream_crc(input, input_len)?;

    if output.seek(SeekFrom::End(0))? != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "TKLZ streaming output must be empty",
        )
        .into());
    }
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&[0_u8; HEADER_LEN])?;
    let mut encoder = StreamRangeEncoder::new(output);
    let mut source = PageCache::new(input, input_len);
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut model = Match8Model::new();
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut literal_start = 0_u64;
    let mut literal_length = 0_usize;
    let mut position = 0_u64;

    while position < input_len {
        if let Some((length, distance)) =
            find_match_stream(&mut source, position, input_len, &table, candidates)?
        {
            if literal_length != 0 {
                encode_literal_stream(
                    &mut encoder,
                    &mut source,
                    &mut model,
                    &mut types,
                    &mut literal_lengths,
                    literal_start,
                    literal_length,
                )?;
                literal_length = 0;
            }
            encode_adaptive_streaming(&mut encoder, &mut types, 1)?;
            encode_bucket_streaming(&mut encoder, &mut match_lengths, length - MIN_MATCH)?;
            encode_bucket_streaming(&mut encoder, &mut distances, distance - 1)?;
            for offset in 0..length {
                let offset = u64::try_from(offset).map_err(|_| LzError::InvalidArtifact)?;
                let byte = source
                    .byte_at(position + offset)?
                    .ok_or(LzError::InvalidArtifact)?;
                model.update(byte);
                insert_stream(&mut source, position + offset, &mut table)?;
            }
            position += u64::try_from(length).map_err(|_| LzError::InvalidArtifact)?;
            literal_start = position;
        } else {
            if literal_length == 0 {
                literal_start = position;
            }
            literal_length = literal_length
                .checked_add(1)
                .ok_or(LzError::InvalidArtifact)?;
            insert_stream(&mut source, position, &mut table)?;
            position += 1;
        }
    }
    if literal_length != 0 {
        encode_literal_stream(
            &mut encoder,
            &mut source,
            &mut model,
            &mut types,
            &mut literal_lengths,
            literal_start,
            literal_length,
        )?;
    }
    encode_adaptive_streaming(&mut encoder, &mut types, 2)?;
    let (output, payload_len) = encoder.finish()?;
    let artifact_len = (HEADER_LEN as u64)
        .checked_add(payload_len)
        .ok_or(LzError::InvalidArtifact)?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(MAGIC)?;
    output.write_all(&[0; 4])?;
    output.write_all(&input_len.to_le_bytes())?;
    output.write_all(&payload_len.to_le_bytes())?;
    output.write_all(&crc.finish().to_le_bytes())?;
    output.flush()?;

    Ok(LzEncodeStats {
        input_bytes: input_len,
        payload_bytes: payload_len,
        artifact_bytes: artifact_len,
    })
}

/// Decodes a complete RFC 0014 research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, invalid arithmetic,
/// invalid match extents, or a checksum mismatch.
pub fn decode_lz_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, LzError> {
    decode_lz_artifact_with_model(artifact, max_output, *MAGIC, StandardMatchModel::new())
}

/// Decodes an RFC 0017 literal-phase research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, invalid arithmetic,
/// invalid match extents, or a checksum mismatch.
pub fn decode_lz_phase_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, LzError> {
    decode_lz_artifact_with_model(artifact, max_output, *PHASE_MAGIC, PhaseMatchModel::new())
}

/// Decodes an RFC 0018 short-match research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, invalid arithmetic,
/// invalid match extents, or a checksum mismatch.
pub fn decode_lz_short_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, LzError> {
    if artifact.len() < HEADER_LEN || artifact[..4] != *SHORT_MAGIC || artifact[4..8] != [0; 4] {
        return Err(LzError::InvalidArtifact);
    }
    let output_len = read_u64(&artifact[8..16]);
    if output_len > max_output {
        return Err(LzError::OutputLimitExceeded);
    }
    let output_len = usize::try_from(output_len).map_err(|_| LzError::InvalidArtifact)?;
    let payload_len =
        usize::try_from(read_u64(&artifact[16..24])).map_err(|_| LzError::InvalidArtifact)?;
    if artifact.len()
        != HEADER_LEN
            .checked_add(payload_len)
            .ok_or(LzError::InvalidArtifact)?
    {
        return Err(LzError::InvalidArtifact);
    }
    let expected_crc = u32::from_le_bytes([artifact[24], artifact[25], artifact[26], artifact[27]]);
    let mut decoder = RangeDecoder::new(&artifact[HEADER_LEN..]);
    let mut model = PhaseMatchModel::new();
    let mut types = AdaptiveDistribution::new(4);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut short_lengths = AdaptiveDistribution::new(5);
    let mut distances = AdaptiveDistribution::new(32);
    let mut output = Vec::with_capacity(output_len);

    loop {
        match decode_adaptive(&mut decoder, &mut types)? {
            0 => {
                let length = decode_bucket(&mut decoder, &mut literal_lengths)?
                    .checked_add(1)
                    .ok_or(LzError::InvalidArtifact)?;
                ensure_remaining(output.len(), length, output_len)?;
                for offset in 0..length {
                    model.set_phase(Some(literal_phase(offset)));
                    let scaled = decoder.scaled(model.total())?;
                    let (byte, low, high, total) = model.symbol(scaled);
                    decoder.advance(low, high, total)?;
                    output.push(byte);
                    model.update(byte);
                }
            }
            1 => {
                let length = decode_bucket(&mut decoder, &mut match_lengths)?
                    .checked_add(MIN_MATCH)
                    .filter(|&length| length <= MAX_MATCH)
                    .ok_or(LzError::InvalidArtifact)?;
                decode_short_copy(
                    &mut decoder,
                    &mut distances,
                    &mut model,
                    &mut output,
                    output_len,
                    length,
                )?;
            }
            2 => {
                let length = decode_adaptive(&mut decoder, &mut short_lengths)? + 3;
                if length > 7 {
                    return Err(LzError::InvalidArtifact);
                }
                decode_short_copy(
                    &mut decoder,
                    &mut distances,
                    &mut model,
                    &mut output,
                    output_len,
                    length,
                )?;
            }
            3 => {
                if output.len() != output_len {
                    return Err(LzError::InvalidArtifact);
                }
                break;
            }
            _ => return Err(LzError::InvalidArtifact),
        }
    }
    if crc32(&output) != expected_crc {
        return Err(LzError::ChecksumMismatch);
    }
    Ok(output)
}

/// Decodes an RFC 0019 structural-block-type research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, invalid arithmetic,
/// invalid match extents, or a checksum mismatch.
pub fn decode_lz_structural_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, LzError> {
    if artifact.len() < HEADER_LEN || artifact[..4] != *STRUCTURAL_MAGIC || artifact[4..8] != [0; 4]
    {
        return Err(LzError::InvalidArtifact);
    }
    let output_len = read_u64(&artifact[8..16]);
    if output_len > max_output {
        return Err(LzError::OutputLimitExceeded);
    }
    let output_len = usize::try_from(output_len).map_err(|_| LzError::InvalidArtifact)?;
    let payload_len =
        usize::try_from(read_u64(&artifact[16..24])).map_err(|_| LzError::InvalidArtifact)?;
    if artifact.len()
        != HEADER_LEN
            .checked_add(payload_len)
            .ok_or(LzError::InvalidArtifact)?
    {
        return Err(LzError::InvalidArtifact);
    }
    let expected_crc = u32::from_le_bytes([artifact[24], artifact[25], artifact[26], artifact[27]]);
    let mut decoder = RangeDecoder::new(&artifact[HEADER_LEN..]);
    let mut model = PhaseMatchModel::new();
    let mut types = AdaptiveDistribution::new(2);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut output = Vec::with_capacity(output_len);
    let mut previous_was_literal = false;

    while output.len() < output_len {
        let block_type = if previous_was_literal {
            1
        } else {
            decode_adaptive(&mut decoder, &mut types)?
        };
        match block_type {
            0 => {
                let length = decode_bucket(&mut decoder, &mut literal_lengths)?
                    .checked_add(1)
                    .ok_or(LzError::InvalidArtifact)?;
                ensure_remaining(output.len(), length, output_len)?;
                for offset in 0..length {
                    model.set_phase(Some(literal_phase(offset)));
                    let scaled = decoder.scaled(model.total())?;
                    let (byte, low, high, total) = model.symbol(scaled);
                    decoder.advance(low, high, total)?;
                    output.push(byte);
                    model.update(byte);
                }
                previous_was_literal = true;
            }
            1 => {
                let length = decode_bucket(&mut decoder, &mut match_lengths)?
                    .checked_add(MIN_MATCH)
                    .filter(|&length| length <= MAX_MATCH)
                    .ok_or(LzError::InvalidArtifact)?;
                decode_short_copy(
                    &mut decoder,
                    &mut distances,
                    &mut model,
                    &mut output,
                    output_len,
                    length,
                )?;
                previous_was_literal = false;
            }
            _ => return Err(LzError::InvalidArtifact),
        }
    }
    if crc32(&output) != expected_crc {
        return Err(LzError::ChecksumMismatch);
    }
    Ok(output)
}

fn decode_short_copy(
    decoder: &mut RangeDecoder<'_>,
    distances: &mut AdaptiveDistribution,
    model: &mut PhaseMatchModel,
    output: &mut Vec<u8>,
    output_len: usize,
    length: usize,
) -> Result<(), LzError> {
    model.set_phase(None);
    let distance = decode_bucket(decoder, distances)?
        .checked_add(1)
        .filter(|&distance| distance <= MAX_DISTANCE && distance <= output.len())
        .ok_or(LzError::InvalidArtifact)?;
    ensure_remaining(output.len(), length, output_len)?;
    for _ in 0..length {
        let byte = output[output.len() - distance];
        output.push(byte);
        model.update(byte);
    }
    Ok(())
}

fn decode_lz_artifact_with_model(
    artifact: &[u8],
    max_output: u64,
    magic: [u8; 4],
    mut model: impl BlockByteModel,
) -> Result<Vec<u8>, LzError> {
    if artifact.len() < HEADER_LEN || artifact[..4] != magic || artifact[4..8] != [0; 4] {
        return Err(LzError::InvalidArtifact);
    }
    let output_len = read_u64(&artifact[8..16]);
    if output_len > max_output {
        return Err(LzError::OutputLimitExceeded);
    }
    let output_len = usize::try_from(output_len).map_err(|_| LzError::InvalidArtifact)?;
    let payload_len =
        usize::try_from(read_u64(&artifact[16..24])).map_err(|_| LzError::InvalidArtifact)?;
    if artifact.len()
        != HEADER_LEN
            .checked_add(payload_len)
            .ok_or(LzError::InvalidArtifact)?
    {
        return Err(LzError::InvalidArtifact);
    }
    let expected_crc = u32::from_le_bytes([artifact[24], artifact[25], artifact[26], artifact[27]]);
    let mut decoder = RangeDecoder::new(&artifact[HEADER_LEN..]);
    let mut types = AdaptiveDistribution::new(3);
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);
    let mut output = Vec::with_capacity(output_len);

    loop {
        let block_type = decode_adaptive(&mut decoder, &mut types)?;
        match block_type {
            0 => {
                let encoded = decode_bucket(&mut decoder, &mut literal_lengths)?;
                let length = encoded.checked_add(1).ok_or(LzError::InvalidArtifact)?;
                ensure_remaining(output.len(), length, output_len)?;
                for offset in 0..length {
                    model.set_phase(Some(literal_phase(offset)));
                    let scaled = decoder.scaled(model.total())?;
                    let (byte, low, high, total) = model.symbol(scaled);
                    decoder.advance(low, high, total)?;
                    output.push(byte);
                    model.update(byte);
                }
            }
            1 => {
                model.set_phase(None);
                let encoded_length = decode_bucket(&mut decoder, &mut match_lengths)?;
                let length = encoded_length
                    .checked_add(MIN_MATCH)
                    .filter(|&length| length <= MAX_MATCH)
                    .ok_or(LzError::InvalidArtifact)?;
                let encoded_distance = decode_bucket(&mut decoder, &mut distances)?;
                let distance = encoded_distance
                    .checked_add(1)
                    .filter(|&distance| distance <= MAX_DISTANCE && distance <= output.len())
                    .ok_or(LzError::InvalidArtifact)?;
                ensure_remaining(output.len(), length, output_len)?;
                for _ in 0..length {
                    let source = output.len() - distance;
                    let byte = output[source];
                    output.push(byte);
                    model.update(byte);
                }
            }
            2 => {
                if output.len() != output_len {
                    return Err(LzError::InvalidArtifact);
                }
                break;
            }
            _ => return Err(LzError::InvalidArtifact),
        }
    }

    if crc32(&output) != expected_crc {
        return Err(LzError::ChecksumMismatch);
    }
    Ok(output)
}

/// Decodes an RFC 0014 artifact with bounded input and output buffers.
///
/// The decoder retains at most the 64 MiB match window plus a 64 KiB output
/// buffer. It does not retain the complete artifact or restored output.
///
/// # Errors
///
/// Returns an error for I/O failure, invalid fields, excessive output, invalid
/// arithmetic, invalid match extents, or a checksum mismatch.
pub fn decode_lz_stream(
    mut artifact: impl Read + Seek,
    output: impl Write,
    max_output: u64,
) -> Result<LzStreamStats, LzStreamError> {
    decode_lz_stream_with_model(
        &mut artifact,
        output,
        max_output,
        *MAGIC,
        StandardMatchModel::new(),
        false,
    )
}

/// Decodes an RFC 0017 literal-phase artifact with bounded memory.
///
/// # Errors
///
/// Returns an error for I/O failure, invalid fields, excessive output, invalid
/// arithmetic, invalid match extents, or a checksum mismatch.
pub fn decode_lz_phase_stream(
    mut artifact: impl Read + Seek,
    output: impl Write,
    max_output: u64,
) -> Result<LzStreamStats, LzStreamError> {
    decode_lz_stream_with_model(
        &mut artifact,
        output,
        max_output,
        *PHASE_MAGIC,
        PhaseMatchModel::new(),
        false,
    )
}

/// Decodes an RFC 0019 structural-block-type artifact with bounded memory.
///
/// # Errors
///
/// Returns an error for I/O failure, invalid fields, excessive output, invalid
/// arithmetic, invalid match extents, or a checksum mismatch.
pub fn decode_lz_structural_stream(
    mut artifact: impl Read + Seek,
    output: impl Write,
    max_output: u64,
) -> Result<LzStreamStats, LzStreamError> {
    decode_lz_stream_with_model(
        &mut artifact,
        output,
        max_output,
        *STRUCTURAL_MAGIC,
        PhaseMatchModel::new(),
        true,
    )
}

fn decode_lz_stream_with_model(
    artifact: &mut (impl Read + Seek),
    output: impl Write,
    max_output: u64,
    magic: [u8; 4],
    mut model: impl BlockByteModel,
    structural_types: bool,
) -> Result<LzStreamStats, LzStreamError> {
    let (output_len, payload_len, expected_crc) = read_stream_header(artifact, max_output, magic)?;
    let window_capacity =
        usize::try_from(output_len.min(MAX_DISTANCE_U64)).map_err(|_| LzError::InvalidArtifact)?;
    let mut restored = StreamOutput::new(output, window_capacity);
    let mut decoder = StreamRangeDecoder::new(artifact.take(payload_len))?;
    let mut types = AdaptiveDistribution::new(if structural_types { 2 } else { 3 });
    let mut literal_lengths = AdaptiveDistribution::new(32);
    let mut match_lengths = AdaptiveDistribution::new(32);
    let mut distances = AdaptiveDistribution::new(32);

    let mut previous_was_literal = false;
    loop {
        if structural_types && restored.bytes == output_len {
            break;
        }
        let block_type = decode_stream_block_type(
            &mut decoder,
            &mut types,
            structural_types,
            previous_was_literal,
        )?;
        match block_type {
            0 => {
                let encoded = decode_bucket_stream(&mut decoder, &mut literal_lengths)?;
                let length = encoded.checked_add(1).ok_or(LzError::InvalidArtifact)?;
                ensure_remaining_u64(restored.bytes, length, output_len)?;
                for offset in 0..length {
                    model.set_phase(Some(literal_phase(offset)));
                    let scaled = decoder.scaled(model.total())?;
                    let (byte, low, high, total) = model.symbol(scaled);
                    decoder.advance(low, high, total)?;
                    restored.push(byte)?;
                    model.update(byte);
                }
                previous_was_literal = true;
            }
            1 => {
                model.set_phase(None);
                let encoded_length = decode_bucket_stream(&mut decoder, &mut match_lengths)?;
                let length = encoded_length
                    .checked_add(MIN_MATCH)
                    .filter(|&length| length <= MAX_MATCH)
                    .ok_or(LzError::InvalidArtifact)?;
                let encoded_distance = decode_bucket_stream(&mut decoder, &mut distances)?;
                let distance = encoded_distance
                    .checked_add(1)
                    .filter(|&distance| {
                        distance <= MAX_DISTANCE
                            && u64::try_from(distance).is_ok_and(|value| value <= restored.bytes)
                    })
                    .ok_or(LzError::InvalidArtifact)?;
                ensure_remaining_u64(restored.bytes, length, output_len)?;
                for _ in 0..length {
                    let byte = restored.match_byte(distance)?;
                    restored.push(byte)?;
                    model.update(byte);
                }
                previous_was_literal = false;
            }
            2 => {
                if structural_types {
                    return Err(LzError::InvalidArtifact.into());
                }
                if restored.bytes != output_len {
                    return Err(LzError::InvalidArtifact.into());
                }
                break;
            }
            _ => return Err(LzError::InvalidArtifact.into()),
        }
    }
    let (bytes, actual_crc) = restored.finish()?;
    if actual_crc != expected_crc {
        return Err(LzError::ChecksumMismatch.into());
    }
    Ok(LzStreamStats {
        output_bytes: bytes,
        payload_bytes: payload_len,
    })
}

fn read_stream_header(
    artifact: &mut (impl Read + Seek),
    max_output: u64,
    magic: [u8; 4],
) -> Result<(u64, u64, u32), LzStreamError> {
    let artifact_len = artifact.seek(SeekFrom::End(0))?;
    artifact.seek(SeekFrom::Start(0))?;
    let mut header = [0_u8; HEADER_LEN];
    artifact.read_exact(&mut header).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            LzStreamError::Artifact(LzError::InvalidArtifact)
        } else {
            LzStreamError::Io(error)
        }
    })?;
    if header[..4] != magic || header[4..8] != [0; 4] {
        return Err(LzError::InvalidArtifact.into());
    }
    let output_len = read_u64(&header[8..16]);
    if output_len > max_output {
        return Err(LzError::OutputLimitExceeded.into());
    }
    let payload_len = read_u64(&header[16..24]);
    if artifact_len
        != 28_u64
            .checked_add(payload_len)
            .ok_or(LzError::InvalidArtifact)?
    {
        return Err(LzError::InvalidArtifact.into());
    }
    let crc = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);
    Ok((output_len, payload_len, crc))
}

fn decode_stream_block_type<R: Read>(
    decoder: &mut StreamRangeDecoder<R>,
    types: &mut AdaptiveDistribution,
    structural_types: bool,
    previous_was_literal: bool,
) -> Result<usize, LzStreamError> {
    if structural_types && previous_was_literal {
        Ok(1)
    } else {
        decode_adaptive_stream(decoder, types)
    }
}

fn ensure_remaining_u64(position: u64, length: usize, output_len: u64) -> Result<(), LzError> {
    let length = u64::try_from(length).map_err(|_| LzError::InvalidArtifact)?;
    if position
        .checked_add(length)
        .filter(|&end| end <= output_len)
        .is_none()
    {
        return Err(LzError::InvalidArtifact);
    }
    Ok(())
}

fn ensure_remaining(position: usize, length: usize, output_len: usize) -> Result<(), LzError> {
    if position
        .checked_add(length)
        .filter(|&end| end <= output_len)
        .is_none()
    {
        return Err(LzError::InvalidArtifact);
    }
    Ok(())
}

struct PageCache<R> {
    input: R,
    input_len: u64,
    pages: Vec<CachedPage>,
    tick: u64,
}

struct CachedPage {
    index: u64,
    bytes: Vec<u8>,
    last_used: u64,
}

impl<R: Read + Seek> PageCache<R> {
    fn new(input: R, input_len: u64) -> Self {
        Self {
            input,
            input_len,
            pages: Vec::with_capacity(INPUT_PAGE_COUNT),
            tick: 0,
        }
    }

    fn byte_at(&mut self, position: u64) -> io::Result<Option<u8>> {
        if position >= self.input_len {
            return Ok(None);
        }
        let page_index = position / INPUT_PAGE_LEN as u64;
        let offset = usize::try_from(position % INPUT_PAGE_LEN as u64)
            .map_err(|_| io::Error::other("TKLZ input page offset exceeds usize"))?;
        self.tick = self.tick.wrapping_add(1);
        if let Some(page) = self.pages.iter_mut().find(|page| page.index == page_index) {
            page.last_used = self.tick;
            return Ok(page.bytes.get(offset).copied());
        }

        let page = {
            let start = page_index * INPUT_PAGE_LEN as u64;
            let length = usize::try_from((self.input_len - start).min(INPUT_PAGE_LEN as u64))
                .map_err(|_| io::Error::other("TKLZ input page length exceeds usize"))?;
            let mut bytes = vec![0_u8; length];
            self.input.seek(SeekFrom::Start(start))?;
            self.input.read_exact(&mut bytes)?;
            CachedPage {
                index: page_index,
                bytes,
                last_used: self.tick,
            }
        };
        if self.pages.len() < INPUT_PAGE_COUNT {
            self.pages.push(page);
        } else {
            let oldest = self
                .pages
                .iter()
                .enumerate()
                .min_by_key(|(_, page)| page.last_used)
                .map(|(index, _)| index)
                .expect("page cache is not empty");
            self.pages[oldest] = page;
        }
        Ok(self
            .pages
            .iter()
            .find(|page| page.index == page_index)
            .and_then(|page| page.bytes.get(offset))
            .copied())
    }
}

fn key_at_stream<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
) -> Result<Option<u64>, LzStreamError> {
    let mut bytes = [0_u8; MIN_MATCH];
    for (offset, byte) in bytes.iter_mut().enumerate() {
        let offset = u64::try_from(offset).map_err(|_| LzError::InvalidArtifact)?;
        let Some(value) = input.byte_at(position + offset)? else {
            return Ok(None);
        };
        *byte = value;
    }
    Ok(Some(u64::from_be_bytes(bytes)))
}

fn insert_stream<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
    table: &mut [Entry],
) -> Result<(), LzStreamError> {
    let Some(key) = key_at_stream(input, position)? else {
        return Ok(());
    };
    let entry = &mut table[table_index(key)];
    let position = u32::try_from(position).map_err(|_| LzError::InvalidArtifact)?;
    if entry.occupied && entry.key == key {
        entry.previous_position = entry.position;
        entry.has_previous = true;
        entry.position = position;
    } else {
        *entry = Entry {
            key,
            position,
            previous_position: 0,
            has_previous: false,
            occupied: true,
        };
    }
    Ok(())
}

fn find_match_stream<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
    input_len: u64,
    table: &[Entry],
    candidates: usize,
) -> Result<Option<(usize, usize)>, LzStreamError> {
    let Some(key) = key_at_stream(input, position)? else {
        return Ok(None);
    };
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return Ok(None);
    }
    let mut best = stream_match_at(input, position, input_len, entry.position)?;
    if candidates == 2 && entry.has_previous {
        if let Some(older) = stream_match_at(input, position, input_len, entry.previous_position)? {
            if best.is_none_or(|latest| older.0 > latest.0) {
                best = Some(older);
            }
        }
    }
    Ok(best)
}

fn stream_match_at<R: Read + Seek>(
    input: &mut PageCache<R>,
    position: u64,
    input_len: u64,
    candidate: u32,
) -> Result<Option<(usize, usize)>, LzStreamError> {
    let candidate = u64::from(candidate);
    let Some(distance) = position
        .checked_sub(candidate)
        .filter(|&value| value != 0 && value <= MAX_DISTANCE_U64)
    else {
        return Ok(None);
    };
    let limit = usize::try_from((input_len - position).min(MAX_MATCH as u64))
        .map_err(|_| LzError::InvalidArtifact)?;
    let mut length = MIN_MATCH;
    while length < limit {
        let offset = u64::try_from(length).map_err(|_| LzError::InvalidArtifact)?;
        if input.byte_at(candidate + offset)? != input.byte_at(position + offset)? {
            break;
        }
        length += 1;
    }
    Ok(Some((
        length,
        usize::try_from(distance).map_err(|_| LzError::InvalidArtifact)?,
    )))
}

#[allow(clippy::too_many_arguments)]
fn encode_literal_stream<W: Write, R: Read + Seek>(
    encoder: &mut StreamRangeEncoder<W>,
    input: &mut PageCache<R>,
    model: &mut Match8Model,
    types: &mut AdaptiveDistribution,
    literal_lengths: &mut AdaptiveDistribution,
    start: u64,
    length: usize,
) -> Result<(), LzStreamError> {
    encode_adaptive_streaming(encoder, types, 0)?;
    encode_bucket_streaming(encoder, literal_lengths, length - 1)?;
    for offset in 0..length {
        let offset = u64::try_from(offset).map_err(|_| LzError::InvalidArtifact)?;
        let byte = input
            .byte_at(start + offset)?
            .ok_or(LzError::InvalidArtifact)?;
        let (low, high, total) = model.range(byte);
        encoder.encode(low, high, total)?;
        model.update(byte);
    }
    Ok(())
}

#[derive(Clone, Copy, Default)]
struct Entry {
    key: u64,
    position: u32,
    previous_position: u32,
    has_previous: bool,
    occupied: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Block {
    Literal { start: usize, length: usize },
    Match { length: usize, distance: usize },
}

fn coalesce_same_distance(blocks: Vec<Block>) -> Vec<Block> {
    let mut result = Vec::with_capacity(blocks.len());
    let mut pending: Option<(usize, usize)> = None;
    for block in blocks {
        match block {
            Block::Match { length, distance } => match pending {
                Some((pending_length, pending_distance)) if pending_distance == distance => {
                    pending = Some((
                        pending_length
                            .checked_add(length)
                            .expect("parsed match lengths fit usize"),
                        distance,
                    ));
                }
                Some((pending_length, pending_distance)) => {
                    append_match_run(&mut result, pending_length, pending_distance);
                    pending = Some((length, distance));
                }
                None => pending = Some((length, distance)),
            },
            literal @ Block::Literal { .. } => {
                if let Some((length, distance)) = pending.take() {
                    append_match_run(&mut result, length, distance);
                }
                result.push(literal);
            }
        }
    }
    if let Some((length, distance)) = pending {
        append_match_run(&mut result, length, distance);
    }
    result
}

fn append_match_run(blocks: &mut Vec<Block>, mut length: usize, distance: usize) {
    while length > MAX_MATCH {
        let remainder = length - MAX_MATCH;
        let chunk = if remainder < MIN_MATCH {
            MAX_MATCH - (MIN_MATCH - remainder)
        } else {
            MAX_MATCH
        };
        blocks.push(Block::Match {
            length: chunk,
            distance,
        });
        length -= chunk;
    }
    debug_assert!((MIN_MATCH..=MAX_MATCH).contains(&length));
    blocks.push(Block::Match { length, distance });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShortBlock {
    Literal { start: usize, length: usize },
    LongMatch { length: usize, distance: usize },
    ShortMatch { length: usize, distance: usize },
}

fn parse(input: &[u8], candidates: usize) -> Vec<Block> {
    let mut table = vec![Entry::default(); TABLE_LEN];
    let mut blocks = Vec::new();
    let mut literal_start = 0;
    let mut literal_length = 0;
    let mut position = 0;
    while position < input.len() {
        if let Some((length, distance)) = find_match(input, position, &table, candidates) {
            if literal_length != 0 {
                blocks.push(Block::Literal {
                    start: literal_start,
                    length: literal_length,
                });
                literal_length = 0;
            }
            blocks.push(Block::Match { length, distance });
            for consumed in position..position + length {
                insert(input, consumed, &mut table);
            }
            position += length;
            literal_start = position;
        } else {
            if literal_length == 0 {
                literal_start = position;
            }
            literal_length += 1;
            insert(input, position, &mut table);
            position += 1;
        }
    }
    if literal_length != 0 {
        blocks.push(Block::Literal {
            start: literal_start,
            length: literal_length,
        });
    }
    blocks
}

fn find_match(
    input: &[u8],
    position: usize,
    table: &[Entry],
    candidates: usize,
) -> Option<(usize, usize)> {
    let key = key_at(input, position)?;
    let entry = table[table_index(key)];
    if !entry.occupied || entry.key != key {
        return None;
    }
    let mut best = buffered_match_at(input, position, entry.position);
    if candidates == 2 && entry.has_previous {
        if let Some(older) = buffered_match_at(input, position, entry.previous_position) {
            if best.is_none_or(|latest| older.0 > latest.0) {
                best = Some(older);
            }
        }
    }
    best
}

fn buffered_match_at(input: &[u8], position: usize, candidate: u32) -> Option<(usize, usize)> {
    let candidate = usize::try_from(candidate).expect("input position fits usize");
    let distance = position.checked_sub(candidate)?;
    if distance == 0 || distance > MAX_DISTANCE {
        return None;
    }
    let limit = MAX_MATCH.min(input.len() - position);
    let mut length = MIN_MATCH;
    while length < limit && input[candidate + length] == input[position + length] {
        length += 1;
    }
    Some((length, distance))
}

fn insert(input: &[u8], position: usize, table: &mut [Entry]) {
    let Some(key) = key_at(input, position) else {
        return;
    };
    let entry = &mut table[table_index(key)];
    let position = u32::try_from(position).expect("encoder input is bounded to u32");
    if entry.occupied && entry.key == key {
        entry.previous_position = entry.position;
        entry.has_previous = true;
        entry.position = position;
    } else {
        *entry = Entry {
            key,
            position,
            previous_position: 0,
            has_previous: false,
            occupied: true,
        };
    }
}

fn key_at(input: &[u8], position: usize) -> Option<u64> {
    let bytes: [u8; 8] = input.get(position..position + 8)?.try_into().ok()?;
    Some(u64::from_be_bytes(bytes))
}

fn table_index(key: u64) -> usize {
    let value = key.wrapping_mul(HASH_MULTIPLIER) >> (64 - TABLE_BITS);
    usize::try_from(value).expect("20-bit index fits usize")
}

struct AdaptiveDistribution {
    counts: Vec<u16>,
    total: u32,
}

impl AdaptiveDistribution {
    fn new(symbols: usize) -> Self {
        Self {
            counts: vec![1; symbols],
            total: u32::try_from(symbols).expect("symbol count fits u32"),
        }
    }

    fn range(&self, symbol: usize) -> (u32, u32, u32) {
        let low = self.counts[..symbol]
            .iter()
            .map(|&count| u32::from(count))
            .sum();
        (low, low + u32::from(self.counts[symbol]), self.total)
    }

    fn symbol(&self, value: u32) -> Option<(usize, u32, u32, u32)> {
        if value >= self.total {
            return None;
        }
        let mut low = 0;
        for (symbol, &count) in self.counts.iter().enumerate() {
            let high = low + u32::from(count);
            if value < high {
                return Some((symbol, low, high, self.total));
            }
            low = high;
        }
        None
    }

    fn update(&mut self, symbol: usize) {
        if self.total >= MAX_TOTAL {
            self.total = 0;
            for count in &mut self.counts {
                *count = (*count).div_ceil(2);
                self.total += u32::from(*count);
            }
        }
        self.counts[symbol] += 1;
        self.total += 1;
    }
}

fn encode_adaptive(
    encoder: &mut RangeEncoder,
    distribution: &mut AdaptiveDistribution,
    symbol: usize,
) {
    let (low, high, total) = distribution.range(symbol);
    encoder.encode(low, high, total);
    distribution.update(symbol);
}

fn encode_adaptive_streaming<W: Write>(
    encoder: &mut StreamRangeEncoder<W>,
    distribution: &mut AdaptiveDistribution,
    symbol: usize,
) -> io::Result<()> {
    let (low, high, total) = distribution.range(symbol);
    encoder.encode(low, high, total)?;
    distribution.update(symbol);
    Ok(())
}

fn decode_adaptive(
    decoder: &mut RangeDecoder<'_>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzError> {
    let scaled = decoder.scaled(distribution.total)?;
    let (symbol, low, high, total) = distribution.symbol(scaled).ok_or(LzError::InvalidPayload)?;
    decoder.advance(low, high, total)?;
    distribution.update(symbol);
    Ok(symbol)
}

fn encode_bucket(
    encoder: &mut RangeEncoder,
    distribution: &mut AdaptiveDistribution,
    value: usize,
) {
    let shifted_value = value + 1;
    let bucket = usize::try_from(usize::BITS - 1 - shifted_value.leading_zeros())
        .expect("bucket fits usize");
    encode_adaptive(encoder, distribution, bucket);
    let base = (1_usize << bucket) - 1;
    let remainder = value - base;
    for shift in (0..bucket).rev() {
        encoder.encode_bit(((remainder >> shift) & 1) != 0);
    }
}

fn encode_bucket_streaming<W: Write>(
    encoder: &mut StreamRangeEncoder<W>,
    distribution: &mut AdaptiveDistribution,
    value: usize,
) -> io::Result<()> {
    let shifted_value = value + 1;
    let bucket = usize::try_from(usize::BITS - 1 - shifted_value.leading_zeros())
        .expect("bucket fits usize");
    encode_adaptive_streaming(encoder, distribution, bucket)?;
    let base = (1_usize << bucket) - 1;
    let remainder = value - base;
    for shift in (0..bucket).rev() {
        encoder.encode_bit(((remainder >> shift) & 1) != 0)?;
    }
    Ok(())
}

fn decode_bucket(
    decoder: &mut RangeDecoder<'_>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzError> {
    let bucket = decode_adaptive(decoder, distribution)?;
    if bucket >= 32 {
        return Err(LzError::InvalidArtifact);
    }
    let base = (1_usize << bucket) - 1;
    let mut remainder = 0_usize;
    for _ in 0..bucket {
        remainder = (remainder << 1) | usize::from(decoder.decode_bit()?);
    }
    base.checked_add(remainder).ok_or(LzError::InvalidArtifact)
}

struct RangeEncoder {
    writer: BitWriter,
    low: u64,
    high: u64,
    pending: usize,
}

impl RangeEncoder {
    fn new() -> Self {
        Self {
            writer: BitWriter::default(),
            low: 0,
            high: RANGE_MAX,
            pending: 0,
        }
    }

    fn encode(&mut self, cumulative_low: u32, cumulative_high: u32, total: u32) {
        debug_assert!(cumulative_low < cumulative_high && cumulative_high <= total);
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        loop {
            if self.high < HALF {
                self.writer.bit_with_pending(false, &mut self.pending);
            } else if self.low >= HALF {
                self.writer.bit_with_pending(true, &mut self.pending);
                self.low -= HALF;
                self.high -= HALF;
            } else if self.low >= QUARTER && self.high < THREE_QUARTERS {
                self.pending += 1;
                self.low -= QUARTER;
                self.high -= QUARTER;
            } else {
                break;
            }
            self.low <<= 1;
            self.high = (self.high << 1) | 1;
        }
    }

    fn encode_bit(&mut self, bit: bool) {
        let low = u32::from(bit);
        self.encode(low, low + 1, 2);
    }

    fn finish(mut self) -> Vec<u8> {
        self.pending += 1;
        self.writer
            .bit_with_pending(self.low >= QUARTER, &mut self.pending);
        self.writer.finish()
    }
}

struct StreamRangeEncoder<W> {
    writer: StreamBitWriter<W>,
    low: u64,
    high: u64,
    pending: usize,
}

impl<W: Write> StreamRangeEncoder<W> {
    fn new(output: W) -> Self {
        Self {
            writer: StreamBitWriter::new(output),
            low: 0,
            high: RANGE_MAX,
            pending: 0,
        }
    }

    fn encode(&mut self, cumulative_low: u32, cumulative_high: u32, total: u32) -> io::Result<()> {
        debug_assert!(cumulative_low < cumulative_high && cumulative_high <= total);
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        loop {
            if self.high < HALF {
                self.writer.bit_with_pending(false, &mut self.pending)?;
            } else if self.low >= HALF {
                self.writer.bit_with_pending(true, &mut self.pending)?;
                self.low -= HALF;
                self.high -= HALF;
            } else if self.low >= QUARTER && self.high < THREE_QUARTERS {
                self.pending += 1;
                self.low -= QUARTER;
                self.high -= QUARTER;
            } else {
                break;
            }
            self.low <<= 1;
            self.high = (self.high << 1) | 1;
        }
        Ok(())
    }

    fn encode_bit(&mut self, bit: bool) -> io::Result<()> {
        let low = u32::from(bit);
        self.encode(low, low + 1, 2)
    }

    fn finish(mut self) -> io::Result<(W, u64)> {
        self.pending += 1;
        self.writer
            .bit_with_pending(self.low >= QUARTER, &mut self.pending)?;
        self.writer.finish()
    }
}

struct RangeDecoder<'a> {
    reader: BitReader<'a>,
    low: u64,
    high: u64,
    code: u64,
}

impl<'a> RangeDecoder<'a> {
    fn new(payload: &'a [u8]) -> Self {
        let mut reader = BitReader::new(payload);
        let mut code = 0;
        for _ in 0..32 {
            code = (code << 1) | u64::from(reader.bit());
        }
        Self {
            reader,
            low: 0,
            high: RANGE_MAX,
            code,
        }
    }

    fn scaled(&self, total: u32) -> Result<u32, LzError> {
        if total == 0 || self.code < self.low || self.code > self.high {
            return Err(LzError::InvalidPayload);
        }
        let width = self.high - self.low + 1;
        let scaled = ((self.code - self.low + 1) * u64::from(total) - 1) / width;
        u32::try_from(scaled).map_err(|_| LzError::InvalidPayload)
    }

    fn advance(
        &mut self,
        cumulative_low: u32,
        cumulative_high: u32,
        total: u32,
    ) -> Result<(), LzError> {
        if cumulative_low >= cumulative_high || cumulative_high > total {
            return Err(LzError::InvalidPayload);
        }
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        loop {
            if self.high < HALF {
            } else if self.low >= HALF {
                self.low -= HALF;
                self.high -= HALF;
                self.code -= HALF;
            } else if self.low >= QUARTER && self.high < THREE_QUARTERS {
                self.low -= QUARTER;
                self.high -= QUARTER;
                self.code -= QUARTER;
            } else {
                break;
            }
            self.low <<= 1;
            self.high = (self.high << 1) | 1;
            self.code = ((self.code << 1) & RANGE_MAX) | u64::from(self.reader.bit());
        }
        Ok(())
    }

    fn decode_bit(&mut self) -> Result<bool, LzError> {
        let scaled = self.scaled(2)?;
        self.advance(scaled, scaled + 1, 2)?;
        Ok(scaled != 0)
    }
}

#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    current: u8,
    used: u8,
}

impl BitWriter {
    fn bit(&mut self, bit: bool) {
        self.current = (self.current << 1) | u8::from(bit);
        self.used += 1;
        if self.used == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.used = 0;
        }
    }

    fn bit_with_pending(&mut self, bit: bool, pending: &mut usize) {
        self.bit(bit);
        for _ in 0..*pending {
            self.bit(!bit);
        }
        *pending = 0;
    }

    fn finish(mut self) -> Vec<u8> {
        if self.used != 0 {
            self.current <<= 8 - self.used;
            self.bytes.push(self.current);
        }
        self.bytes
    }
}

struct StreamBitWriter<W> {
    output: W,
    buffer: Vec<u8>,
    current: u8,
    used: u8,
    bytes: u64,
}

impl<W: Write> StreamBitWriter<W> {
    fn new(output: W) -> Self {
        Self {
            output,
            buffer: Vec::with_capacity(INPUT_PAGE_LEN),
            current: 0,
            used: 0,
            bytes: 0,
        }
    }

    fn bit(&mut self, bit: bool) -> io::Result<()> {
        self.current = (self.current << 1) | u8::from(bit);
        self.used += 1;
        if self.used == 8 {
            self.buffer.push(self.current);
            self.bytes += 1;
            self.current = 0;
            self.used = 0;
            if self.buffer.len() == self.buffer.capacity() {
                self.flush()?;
            }
        }
        Ok(())
    }

    fn bit_with_pending(&mut self, bit: bool, pending: &mut usize) -> io::Result<()> {
        self.bit(bit)?;
        for _ in 0..*pending {
            self.bit(!bit)?;
        }
        *pending = 0;
        Ok(())
    }

    fn finish(mut self) -> io::Result<(W, u64)> {
        if self.used != 0 {
            self.current <<= 8 - self.used;
            self.buffer.push(self.current);
            self.bytes += 1;
        }
        self.flush()?;
        Ok((self.output, self.bytes))
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }
}

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_index: usize,
}

impl<'a> BitReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bit_index: 0,
        }
    }

    fn bit(&mut self) -> u8 {
        let byte = self.bytes.get(self.bit_index / 8).copied().unwrap_or(0);
        let bit = (byte >> (7 - self.bit_index % 8)) & 1;
        self.bit_index += 1;
        bit
    }
}

fn decode_adaptive_stream<R: Read>(
    decoder: &mut StreamRangeDecoder<R>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzStreamError> {
    let scaled = decoder.scaled(distribution.total)?;
    let (symbol, low, high, total) = distribution.symbol(scaled).ok_or(LzError::InvalidPayload)?;
    decoder.advance(low, high, total)?;
    distribution.update(symbol);
    Ok(symbol)
}

fn decode_bucket_stream<R: Read>(
    decoder: &mut StreamRangeDecoder<R>,
    distribution: &mut AdaptiveDistribution,
) -> Result<usize, LzStreamError> {
    let bucket = decode_adaptive_stream(decoder, distribution)?;
    if bucket >= 32 {
        return Err(LzError::InvalidArtifact.into());
    }
    let base = (1_usize << bucket) - 1;
    let mut remainder = 0_usize;
    for _ in 0..bucket {
        remainder = (remainder << 1) | usize::from(decoder.decode_bit()?);
    }
    base.checked_add(remainder)
        .ok_or(LzStreamError::Artifact(LzError::InvalidArtifact))
}

struct StreamRangeDecoder<R> {
    reader: StreamBitReader<R>,
    low: u64,
    high: u64,
    code: u64,
}

impl<R: Read> StreamRangeDecoder<R> {
    fn new(input: R) -> Result<Self, io::Error> {
        let mut reader = StreamBitReader::new(input);
        let mut code = 0;
        for _ in 0..32 {
            code = (code << 1) | u64::from(reader.bit()?);
        }
        Ok(Self {
            reader,
            low: 0,
            high: RANGE_MAX,
            code,
        })
    }

    fn scaled(&self, total: u32) -> Result<u32, LzError> {
        if total == 0 || self.code < self.low || self.code > self.high {
            return Err(LzError::InvalidPayload);
        }
        let width = self.high - self.low + 1;
        let scaled = ((self.code - self.low + 1) * u64::from(total) - 1) / width;
        u32::try_from(scaled).map_err(|_| LzError::InvalidPayload)
    }

    fn advance(
        &mut self,
        cumulative_low: u32,
        cumulative_high: u32,
        total: u32,
    ) -> Result<(), LzStreamError> {
        if cumulative_low >= cumulative_high || cumulative_high > total {
            return Err(LzError::InvalidPayload.into());
        }
        let width = self.high - self.low + 1;
        self.high = self.low + width * u64::from(cumulative_high) / u64::from(total) - 1;
        self.low += width * u64::from(cumulative_low) / u64::from(total);
        self.renormalize()?;
        Ok(())
    }

    #[inline]
    fn renormalize(&mut self) -> Result<(), LzStreamError> {
        let matching = (self.low ^ self.high).leading_zeros() - 32;
        if matching != 0 {
            let fill = (1_u64 << matching) - 1;
            self.low = (self.low << matching) & RANGE_MAX;
            self.high = ((self.high << matching) | fill) & RANGE_MAX;
            self.code = ((self.code << matching) & RANGE_MAX) | self.reader.bits(matching)?;
        }
        let straddle = ((self.low & !self.high) << 1) & RANGE_MAX;
        let quarter = (straddle << 32).leading_ones();
        if quarter != 0 {
            let fill = (1_u64 << quarter) - 1;
            self.low = ((self.low << quarter) & RANGE_MAX) ^ HALF;
            self.high = (((self.high << quarter) | fill) & RANGE_MAX) ^ HALF;
            self.code = (((self.code << quarter) & RANGE_MAX) | self.reader.bits(quarter)?) ^ HALF;
        }
        Ok(())
    }

    fn decode_bit(&mut self) -> Result<bool, LzStreamError> {
        let scaled = self.scaled(2)?;
        self.advance(scaled, scaled + 1, 2)?;
        Ok(scaled != 0)
    }
}

struct StreamBitReader<R> {
    input: R,
    buffer: Box<[u8]>,
    position: usize,
    filled: usize,
    current: u64,
    remaining: u32,
}

impl<R: Read> StreamBitReader<R> {
    fn new(input: R) -> Self {
        Self {
            input,
            buffer: vec![0; 64 * 1024].into_boxed_slice(),
            position: 0,
            filled: 0,
            current: 0,
            remaining: 0,
        }
    }

    fn refill(&mut self) -> io::Result<()> {
        if self.filled - self.position >= 8 {
            let bytes: [u8; 8] = self.buffer[self.position..self.position + 8]
                .try_into()
                .expect("slice has eight bytes");
            self.position += 8;
            self.current = u64::from_be_bytes(bytes);
            self.remaining = 64;
            return Ok(());
        }
        let byte = self.next_byte()?;
        self.current = u64::from(byte) << 56;
        self.remaining = 8;
        Ok(())
    }

    fn next_byte(&mut self) -> io::Result<u8> {
        if self.position == self.filled {
            self.position = 0;
            self.filled = loop {
                match self.input.read(&mut self.buffer) {
                    Ok(count) => break count,
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            };
            if self.filled == 0 {
                return Ok(0);
            }
        }
        let byte = self.buffer[self.position];
        self.position += 1;
        Ok(byte)
    }

    #[inline]
    fn bits(&mut self, count: u32) -> io::Result<u64> {
        debug_assert!(count <= 32);
        if count <= self.remaining {
            let value = self.current.checked_shr(64 - count).unwrap_or(0);
            self.current = self.current.checked_shl(count).unwrap_or(0);
            self.remaining -= count;
            return Ok(value);
        }
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u64::from(self.bit()?);
        }
        Ok(value)
    }

    #[inline]
    fn bit(&mut self) -> io::Result<u8> {
        if self.remaining == 0 {
            self.refill()?;
        }
        let bit = u8::try_from(self.current >> 63).expect("one bit fits u8");
        self.current <<= 1;
        self.remaining -= 1;
        Ok(bit)
    }
}

struct StreamOutput<W> {
    output: W,
    buffer: Vec<u8>,
    window: VecDeque<u8>,
    window_capacity: usize,
    crc: Crc32,
    bytes: u64,
}

impl<W: Write> StreamOutput<W> {
    fn new(output: W, window_capacity: usize) -> Self {
        Self {
            output,
            buffer: Vec::with_capacity(64 * 1024),
            window: VecDeque::with_capacity(window_capacity),
            window_capacity,
            crc: Crc32::new(),
            bytes: 0,
        }
    }

    fn push(&mut self, byte: u8) -> io::Result<()> {
        self.buffer.push(byte);
        self.crc.update(byte);
        if self.window.len() == self.window_capacity && self.window_capacity != 0 {
            self.window.pop_front();
        }
        if self.window_capacity != 0 {
            self.window.push_back(byte);
        }
        self.bytes = self
            .bytes
            .checked_add(1)
            .ok_or_else(|| io::Error::other("TKLZ output length exceeds u64"))?;
        if self.buffer.len() == self.buffer.capacity() {
            self.flush()?;
        }
        Ok(())
    }

    fn match_byte(&self, distance: usize) -> Result<u8, LzError> {
        let index = self
            .window
            .len()
            .checked_sub(distance)
            .ok_or(LzError::InvalidArtifact)?;
        self.window
            .get(index)
            .copied()
            .ok_or(LzError::InvalidArtifact)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.write_all(&self.buffer)?;
        self.buffer.clear();
        Ok(())
    }

    fn finish(mut self) -> io::Result<(u64, u32)> {
        self.flush()?;
        self.output.flush()?;
        Ok((self.bytes, self.crc.finish()))
    }
}

#[derive(Clone, Copy)]
struct Crc32(u32);

impl Crc32 {
    const fn new() -> Self {
        Self(u32::MAX)
    }

    fn update(&mut self, byte: u8) {
        self.0 ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(self.0 & 1);
            self.0 = (self.0 >> 1) ^ (0xedb8_8320 & mask);
        }
    }

    fn update_slice(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.update(byte);
        }
    }

    const fn finish(self) -> u32 {
        !self.0
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        Block, EXPERT_DIVISORS, EXPERTS, IntegerExpertMixture, LzError, coalesce_same_distance,
        decode_lz_artifact, decode_lz_phase_artifact, decode_lz_phase_stream,
        decode_lz_short_artifact, decode_lz_stream, decode_lz_structural_artifact,
        decode_lz_structural_stream, encode_lz_artifact, encode_lz_associative_artifact,
        encode_lz_coalesced_artifact, encode_lz_phase_artifact, encode_lz_phase_stream,
        encode_lz_priced_artifact, encode_lz_short_artifact, encode_lz_stream,
        encode_lz_structural_artifact, encode_lz_structural_repriced_artifact,
        encode_lz_structural_stream, encode_lz_two_candidate_artifact,
        encode_lz_two_candidate_stream, literal_phase, profile_length_conditioned_distance,
        profile_literal_phase_order2, profile_lz_priced_residual, profile_match_distance_literal,
        short_match,
    };
    use crate::HEADER_LEN;

    #[test]
    fn bulk_renormalization_matches_single_bit_steps() {
        use super::{
            HALF, QUARTER, RANGE_MAX, StreamBitReader, StreamRangeDecoder, THREE_QUARTERS,
        };

        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let payload: Vec<u8> = (0..4_096).map(|_| next().to_le_bytes()[0]).collect();
        for case in 0..20_000 {
            let mut a = next() & RANGE_MAX;
            let mut b = next() & RANGE_MAX;
            if case % 3 == 0 {
                let shift = next() % 32;
                b = (a & !((1 << shift) - 1)) | (b & ((1 << shift) - 1));
                if case % 2 == 0 {
                    a = (a & !THREE_QUARTERS) | QUARTER;
                    b = (b & !THREE_QUARTERS) | HALF;
                }
            }
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            let code = low + next() % (high - low + 1);
            let offset = usize::try_from(next() % 4_000).unwrap();
            let mut bulk = StreamRangeDecoder {
                reader: StreamBitReader::new(Cursor::new(&payload[offset..])),
                low,
                high,
                code,
            };
            bulk.renormalize().unwrap();

            let mut reader = StreamBitReader::new(Cursor::new(&payload[offset..]));
            let (mut low, mut high, mut code) = (low, high, code);
            loop {
                if high < HALF {
                } else if low >= HALF {
                    low -= HALF;
                    high -= HALF;
                    code -= HALF;
                } else if low >= QUARTER && high < THREE_QUARTERS {
                    low -= QUARTER;
                    high -= QUARTER;
                    code -= QUARTER;
                } else {
                    break;
                }
                low <<= 1;
                high = (high << 1) | 1;
                code = ((code << 1) & RANGE_MAX) | u64::from(reader.bit().unwrap());
            }
            assert_eq!(
                (bulk.low, bulk.high, bulk.code),
                (low, high, code),
                "case {case}"
            );
            assert_eq!(
                bulk.reader.bit().unwrap(),
                reader.bit().unwrap(),
                "case {case}"
            );
        }
    }

    #[test]
    fn priced_artifact_round_trips_through_existing_decoders() {
        let mut input = Vec::new();
        for round in 0..3_000_u32 {
            input.extend_from_slice(b"price-based parsing keeps TKLZ ");
            input.extend_from_slice(&(round % 97).to_le_bytes());
        }
        input.extend(std::iter::repeat_n(7_u8, 5_000));
        let artifact = encode_lz_priced_artifact(&input);
        let limit = input.len() as u64;
        assert_eq!(decode_lz_artifact(&artifact, limit).unwrap(), input);
        let mut restored = Vec::new();
        decode_lz_stream(Cursor::new(&artifact), &mut restored, limit).unwrap();
        assert_eq!(restored, input);
    }

    #[test]
    fn priced_artifact_handles_empty_and_tiny_inputs() {
        for input in [&b""[..], b"a", b"abcdefg", b"abcdefgh"] {
            let artifact = encode_lz_priced_artifact(input);
            assert_eq!(decode_lz_artifact(&artifact, 8).unwrap(), input);
        }
    }

    #[test]
    fn phase_artifact_round_trips_and_rejects_other_magic() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXabcdefghYabcdefghZ".repeat(2_000),
        ] {
            let artifact = encode_lz_phase_artifact(&input);
            assert_eq!(
                decode_lz_phase_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            assert!(decode_lz_artifact(&artifact, u64::MAX).is_err());
            if !artifact.is_empty() {
                let mut corrupt = artifact.clone();
                let index = corrupt.len() / 2;
                corrupt[index] ^= 0x20;
                assert!(decode_lz_phase_artifact(&corrupt, input.len() as u64).is_err());
            }
        }
    }

    #[test]
    fn short_match_artifact_round_trips_and_is_format_isolated() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcXabcYabcZabcQabcRabcS".repeat(2_000),
        ] {
            let artifact = encode_lz_short_artifact(&input);
            assert_eq!(
                decode_lz_short_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            assert!(decode_lz_artifact(&artifact, u64::MAX).is_err());
            assert!(decode_lz_phase_artifact(&artifact, u64::MAX).is_err());
        }
    }

    #[test]
    fn short_match_artifact_enforces_limits_and_integrity() {
        let input = b"abcXabcYabcZabcQabcRabcS".repeat(2_000);
        let artifact = encode_lz_short_artifact(&input);
        assert_eq!(
            decode_lz_short_artifact(&artifact, 1),
            Err(LzError::OutputLimitExceeded)
        );

        let mut corrupt = artifact;
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        assert!(decode_lz_short_artifact(&corrupt, input.len() as u64).is_err());
    }

    #[test]
    fn structural_artifact_round_trips_and_is_format_isolated() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXabcdefghYabcdefghZ".repeat(2_000),
        ] {
            let artifact = encode_lz_structural_artifact(&input);
            assert_eq!(
                decode_lz_structural_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            assert!(decode_lz_artifact(&artifact, u64::MAX).is_err());
            assert!(decode_lz_phase_artifact(&artifact, u64::MAX).is_err());
        }
    }

    #[test]
    fn associative_artifact_uses_existing_decoder() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXijklmnopYabcdefghZijklmnopQ".repeat(4_000),
        ] {
            let artifact = encode_lz_associative_artifact(&input);
            assert_eq!(
                decode_lz_structural_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn same_distance_coalescing_rebalances_short_remainder() {
        let blocks = vec![
            Block::Match {
                length: 40_000,
                distance: 17,
            },
            Block::Match {
                length: 25_536,
                distance: 17,
            },
            Block::Literal {
                start: 65_536,
                length: 1,
            },
            Block::Match {
                length: 8,
                distance: 17,
            },
        ];
        assert_eq!(
            coalesce_same_distance(blocks),
            vec![
                Block::Match {
                    length: 65_528,
                    distance: 17,
                },
                Block::Match {
                    length: 8,
                    distance: 17,
                },
                Block::Literal {
                    start: 65_536,
                    length: 1,
                },
                Block::Match {
                    length: 8,
                    distance: 17,
                },
            ]
        );
    }

    #[test]
    fn coalesced_artifact_uses_existing_decoder() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXabcdefghYabcdefghZ".repeat(10_000),
        ] {
            let artifact = encode_lz_coalesced_artifact(&input);
            assert_eq!(
                decode_lz_structural_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn structurally_repriced_artifact_uses_existing_decoder() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXabcdefghYabcdefghZ".repeat(2_000),
        ] {
            let artifact = encode_lz_structural_repriced_artifact(&input);
            assert_eq!(
                decode_lz_structural_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn structural_artifact_enforces_limits_and_integrity() {
        let input = b"abcdefghXabcdefghYabcdefghZ".repeat(2_000);
        let artifact = encode_lz_structural_artifact(&input);
        assert_eq!(
            decode_lz_structural_artifact(&artifact, 1),
            Err(LzError::OutputLimitExceeded)
        );

        let mut corrupt = artifact;
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        assert!(decode_lz_structural_artifact(&corrupt, input.len() as u64).is_err());
    }

    #[test]
    fn integer_expert_mixture_matches_reference_arithmetic() {
        let mut mixture = IntegerExpertMixture {
            weights: [1, 3, 17, 257, 4_099, 65_535],
        };
        let total = 16_384_u32;
        let bonuses = EXPERT_DIVISORS
            .map(|divisor| (divisor != u32::MAX).then(|| total / divisor).unwrap_or(0));
        let scaled = std::array::from_fn::<u128, EXPERTS, _>(|index| {
            u128::from(mixture.weights[index]) * (1_u128 << 32) / u128::from(total + bonuses[index])
        });
        let expected_bonus = scaled
            .iter()
            .zip(bonuses)
            .map(|(weight, bonus)| weight * u128::from(bonus))
            .sum::<u128>()
            / scaled.iter().sum::<u128>();
        assert_eq!(u128::from(mixture.bonus(total)), expected_bonus);

        let actual_width = 7_919_u32;
        let hit = true;
        let scores = std::array::from_fn::<u128, EXPERTS, _>(|index| {
            let width = actual_width + u32::from(hit) * bonuses[index];
            u128::from(mixture.weights[index]) * u128::from(width) * (1_u128 << 32)
                / u128::from(total + bonuses[index])
        });
        let score_total = scores.iter().sum::<u128>();
        let expected_weights =
            scores.map(|score| u32::try_from((score * 65_535 / score_total).max(1)).unwrap());
        mixture.update(actual_width, total, hit);
        assert_eq!(mixture.weights, expected_weights);
        assert_eq!(
            [
                literal_phase(0),
                literal_phase(1),
                literal_phase(4),
                literal_phase(16)
            ],
            [0, 1, 2, 3]
        );
    }

    #[test]
    fn bounded_phase_codec_matches_buffered_artifact() {
        let mut input = b"literal phase bounded identity ".repeat(4_000);
        input.extend((0_u8..=255).cycle().take(70_000));
        input.extend_from_slice(&input.clone()[20_000..80_000]);
        let expected = encode_lz_phase_artifact(&input);
        let mut artifact = Cursor::new(Vec::new());
        let stats = encode_lz_phase_stream(Cursor::new(&input), &mut artifact).unwrap();
        assert_eq!(artifact.get_ref(), &expected);
        assert_eq!(stats.artifact_bytes, expected.len() as u64);

        artifact.set_position(0);
        let mut restored = Vec::new();
        decode_lz_phase_stream(artifact, &mut restored, input.len() as u64).unwrap();
        assert_eq!(restored, input);

        let mut rejected = Vec::new();
        assert!(decode_lz_phase_stream(Cursor::new(&expected), &mut rejected, 1).is_err());
        let mut corrupt = expected;
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        assert!(
            decode_lz_phase_stream(Cursor::new(corrupt), &mut Vec::new(), input.len() as u64)
                .is_err()
        );
    }

    #[test]
    fn bounded_structural_codec_matches_buffered_artifact() {
        let mut input = b"structural bounded identity ".repeat(4_000);
        input.extend((0_u8..=255).cycle().take(70_000));
        input.extend_from_slice(&input.clone()[20_000..80_000]);
        let expected = encode_lz_structural_artifact(&input);
        let mut artifact = Cursor::new(Vec::new());
        let stats = encode_lz_structural_stream(Cursor::new(&input), &mut artifact).unwrap();
        assert_eq!(artifact.get_ref(), &expected);
        assert_eq!(stats.artifact_bytes, expected.len() as u64);

        artifact.set_position(0);
        let mut restored = Vec::new();
        decode_lz_structural_stream(artifact, &mut restored, input.len() as u64).unwrap();
        assert_eq!(restored, input);

        let mut rejected = Vec::new();
        assert!(decode_lz_structural_stream(Cursor::new(&expected), &mut rejected, 1).is_err());
        let mut corrupt = expected;
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        assert!(
            decode_lz_structural_stream(Cursor::new(corrupt), &mut Vec::new(), input.len() as u64)
                .is_err()
        );
    }

    #[test]
    fn priced_residual_profile_covers_input() {
        let input = b"abcdefghXabcdefghYabcdefghZ".repeat(1_000);
        let profile = profile_lz_priced_residual(&input);
        assert_eq!(profile.literal_bytes + profile.match_bytes, input.len());
        assert!(profile.global_type_bits.is_finite());
        assert!(profile.structural_type_bits.is_finite());
        assert!(profile.global_type_bits > profile.structural_type_bits);
        assert_eq!(
            profile.structural_type_decisions + profile.implicit_match_types,
            profile.literal_blocks + profile.match_blocks
        );
        assert!(profile.match_length_bits.is_finite());
        assert!(profile.state_match_length_bits.is_finite());
        assert!(profile.distance_bits.is_finite());
        assert!(profile.state_distance_bits.is_finite());
        assert!(profile.distance_remainder_bits > 0);
        assert!(profile.global_remainder_model_bits.is_finite());
        assert!(profile.bucket_remainder_model_bits.is_finite());
        assert!(profile.exclusion_saved_bits.is_finite());
    }

    #[test]
    fn literal_phase_order2_profile_covers_literal_parse() {
        let input = b"abcdefghXabcdefghYabcdefghZ".repeat(1_000);
        let profile = profile_literal_phase_order2(&input);
        let established = profile_lz_priced_residual(&input);
        assert_eq!(profile.input_bytes, input.len());
        assert_eq!(
            profile.literal_bytes_by_phase.iter().sum::<usize>(),
            profile.literal_bytes
        );
        assert!(profile.literal_bytes != 0);
        assert!(profile.control_bits.is_finite());
        assert!(profile.candidate_bits.is_finite());
        assert!(profile.control_bits > 0.0);
        assert!(profile.candidate_bits > 0.0);
        assert!((profile.control_bits - established.integer_phase_literal_bits).abs() < 1e-6);
    }

    #[test]
    fn match_distance_literal_profile_uses_established_control() {
        let input = b"abcdefghXabcdefghYabcdefghZ".repeat(1_000);
        let profile = profile_match_distance_literal(&input);
        let established = profile_lz_priced_residual(&input);
        assert_eq!(profile.input_bytes, input.len());
        assert_eq!(profile.literal_bytes, established.literal_bytes);
        assert!(profile.eligible_literals != 0);
        assert!(profile.source_hits <= profile.eligible_literals);
        assert!(profile.candidate_bits.is_finite());
        assert!((profile.control_bits - established.integer_phase_literal_bits).abs() < 1e-6);
    }

    #[test]
    fn length_conditioned_distance_profile_uses_established_control() {
        let input = b"abcdefghXabcdefghYabcdefghZ".repeat(1_000);
        let profile = profile_length_conditioned_distance(&input);
        let established = profile_lz_priced_residual(&input);
        assert_eq!(profile.input_bytes, input.len());
        assert_eq!(profile.match_blocks, established.match_blocks);
        assert_eq!(
            profile.matches_by_class.iter().sum::<usize>(),
            profile.match_blocks
        );
        assert!(profile.candidate_bits.is_finite());
        assert!((profile.control_bits - established.distance_bits).abs() < 1e-6);
    }

    #[test]
    fn priced_residual_profile_finds_short_literal_matches() {
        let input = b"abcXabcYabcZabcQabcRabcS".repeat(200);
        let profile = profile_lz_priced_residual(&input);
        assert!(profile.short_match_events > 0);
        assert!(profile.short_match_bytes >= profile.short_match_events * 3);
        assert!(profile.short_match_literal_bits > profile.short_match_token_bits);
    }

    #[test]
    fn short_match_does_not_cross_a_literal_boundary() {
        let input = b"abcXabc";
        let mut table = vec![super::ShortEntry::default(); super::TABLE_LEN];
        super::insert_short(input, 0, &mut table);
        assert_eq!(short_match(input, 4, 6, &table), None);
        assert_eq!(short_match(input, 4, 7, &table), Some((3, 4)));
    }

    #[test]
    fn artifacts_round_trip_short_and_repeated_inputs() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefgh01234567abcdefgh01234567".repeat(1_000),
            (0_u8..=255).cycle().take(20_000).collect(),
        ] {
            let artifact = encode_lz_artifact(&input);
            assert_eq!(
                decode_lz_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            if !input.is_empty() {
                assert_eq!(
                    decode_lz_artifact(&artifact, input.len() as u64 - 1),
                    Err(LzError::OutputLimitExceeded)
                );
            }
        }
    }

    #[test]
    fn corruption_and_truncation_are_rejected() {
        let input = b"abcdefgh01234567abcdefgh01234567".repeat(200);
        let artifact = encode_lz_artifact(&input);
        for end in 0..artifact.len() {
            assert!(decode_lz_artifact(&artifact[..end], u64::MAX).is_err());
        }
        for index in [0, 8, 16, 24, artifact.len() / 2] {
            let mut corrupt = artifact.clone();
            corrupt[index] ^= 0x40;
            assert!(decode_lz_artifact(&corrupt, u64::MAX).is_err());
        }
    }

    #[test]
    fn streaming_decode_matches_buffered_decode() {
        let input = b"abcdefgh01234567abcdefgh01234567".repeat(5_000);
        let artifact = encode_lz_artifact(&input);
        let mut output = Vec::new();
        let stats = decode_lz_stream(
            Cursor::new(&artifact),
            &mut output,
            u64::try_from(input.len()).unwrap(),
        )
        .unwrap();
        assert_eq!(output, input);
        assert_eq!(stats.output_bytes, u64::try_from(input.len()).unwrap());
        assert_eq!(
            stats.payload_bytes,
            u64::try_from(artifact.len() - HEADER_LEN).unwrap()
        );

        let mut rejected = Vec::new();
        assert!(decode_lz_stream(Cursor::new(&artifact), &mut rejected, 1).is_err());
    }

    #[test]
    fn streaming_encode_is_byte_identical() {
        let cases = [
            Vec::new(),
            b"short input".to_vec(),
            b"abcdefgh01234567abcdefgh01234567".repeat(5_000),
        ];
        for input in cases {
            let expected = encode_lz_artifact(&input);
            let mut output = Cursor::new(Vec::new());
            let stats = encode_lz_stream(Cursor::new(&input), &mut output).unwrap();
            assert_eq!(output.into_inner(), expected);
            assert_eq!(stats.input_bytes, u64::try_from(input.len()).unwrap());
            assert_eq!(stats.artifact_bytes, u64::try_from(expected.len()).unwrap());
        }

        let mut nonempty = Cursor::new(vec![0_u8]);
        assert!(encode_lz_stream(Cursor::new(b"input"), &mut nonempty).is_err());
    }

    #[test]
    fn two_candidate_artifacts_round_trip_and_stream_identically() {
        let input = b"abcdefgh22222222abcdefgh11111111abcdefgh22222222".repeat(1_000);
        let artifact = encode_lz_two_candidate_artifact(&input);
        assert_eq!(
            decode_lz_artifact(&artifact, u64::try_from(input.len()).unwrap()).unwrap(),
            input
        );

        let mut streamed = Cursor::new(Vec::new());
        encode_lz_two_candidate_stream(Cursor::new(&input), &mut streamed).unwrap();
        assert_eq!(streamed.into_inner(), artifact);
    }
}
