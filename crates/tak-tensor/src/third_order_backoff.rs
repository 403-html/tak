use std::num::NonZeroU16;

use tak_context::Order3BackoffModel;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKB3";

/// Encodes a complete adaptive order-3 backoff research artifact.
#[must_use]
pub fn encode_order3_backoff_artifact(input: &[u8], min_observations: NonZeroU16) -> Vec<u8> {
    let payload = tak_entropy::encode(input, Order3BackoffModel::new(min_observations));
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0, 0]);
    output.extend_from_slice(&min_observations.get().to_le_bytes());
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete adaptive order-3 backoff research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_order3_backoff_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..6] != [0, 0] {
        return Err(Error::InvalidArtifact);
    }
    let min_observations = NonZeroU16::new(u16::from_le_bytes([artifact[6], artifact[7]]))
        .ok_or(Error::InvalidArtifact)?;
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
    let model = Order3BackoffModel::new(min_observations);
    let output = tak_entropy::decode(&artifact[HEADER_LEN..], output_len, model)
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::{decode_order3_backoff_artifact, encode_order3_backoff_artifact};

    #[test]
    fn every_screening_threshold_round_trips() {
        let input = b"abcXabcYabcXabcY".repeat(100);
        for threshold in [2, 4, 8, 16, 32, 64] {
            let artifact = encode_order3_backoff_artifact(
                &input,
                NonZeroU16::new(threshold).expect("threshold is nonzero"),
            );
            assert_eq!(
                decode_order3_backoff_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn rejects_invalid_fields_corruption_and_output_excess() {
        let threshold = NonZeroU16::new(4).unwrap();
        for input in [b"".as_slice(), b"a", b"ab", b"abc"] {
            let artifact = encode_order3_backoff_artifact(input, threshold);
            assert_eq!(
                decode_order3_backoff_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
        let input = b"backoff validation".repeat(20);
        let artifact = encode_order3_backoff_artifact(&input, threshold);
        assert!(decode_order3_backoff_artifact(&artifact, 1).is_err());
        assert!(decode_order3_backoff_artifact(&artifact[..artifact.len() - 1], u64::MAX).is_err());
        let mut zero_threshold = artifact.clone();
        zero_threshold[6..8].copy_from_slice(&0_u16.to_le_bytes());
        assert!(decode_order3_backoff_artifact(&zero_threshold, u64::MAX).is_err());
        let mut corrupt = artifact;
        corrupt[24] ^= 1;
        assert!(decode_order3_backoff_artifact(&corrupt, u64::MAX).is_err());
    }
}
