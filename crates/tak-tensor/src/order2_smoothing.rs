use std::num::NonZeroU16;

use tak_context::Order2SmoothedModel;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKS2";
const WEIGHTS: [u16; 7] = [1, 2, 4, 8, 16, 32, 64];

/// Encodes a complete hierarchical order-2 research artifact.
///
/// # Panics
///
/// Panics if `parent_weight` is not one of the RFC 0005 screening weights.
#[must_use]
pub fn encode_order2_smoothed_artifact(input: &[u8], parent_weight: NonZeroU16) -> Vec<u8> {
    assert!(WEIGHTS.contains(&parent_weight.get()));
    let payload = tak_entropy::encode(input, Order2SmoothedModel::new(parent_weight));
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0, 0]);
    output.extend_from_slice(&parent_weight.get().to_le_bytes());
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete hierarchical order-2 research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_order2_smoothed_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..6] != [0, 0] {
        return Err(Error::InvalidArtifact);
    }
    let weight = u16::from_le_bytes([artifact[6], artifact[7]]);
    if !WEIGHTS.contains(&weight) {
        return Err(Error::InvalidArtifact);
    }
    let parent_weight = NonZeroU16::new(weight).ok_or(Error::InvalidArtifact)?;
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
    let model = Order2SmoothedModel::new(parent_weight);
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

    use super::{decode_order2_smoothed_artifact, encode_order2_smoothed_artifact};

    #[test]
    fn every_screening_weight_round_trips() {
        let input = b"abXabYabXabY".repeat(100);
        for weight in [1, 2, 4, 8, 16, 32, 64] {
            let artifact = encode_order2_smoothed_artifact(
                &input,
                NonZeroU16::new(weight).expect("weight is nonzero"),
            );
            assert_eq!(
                decode_order2_smoothed_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn rejects_invalid_fields_corruption_and_output_excess() {
        let weight = NonZeroU16::new(4).unwrap();
        for input in [b"".as_slice(), b"a", b"ab", b"abc"] {
            let artifact = encode_order2_smoothed_artifact(input, weight);
            assert_eq!(
                decode_order2_smoothed_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
        let input = b"hierarchical smoothing".repeat(20);
        let artifact = encode_order2_smoothed_artifact(&input, weight);
        assert!(decode_order2_smoothed_artifact(&artifact, 1).is_err());
        assert!(
            decode_order2_smoothed_artifact(&artifact[..artifact.len() - 1], u64::MAX).is_err()
        );
        let mut invalid_weight = artifact.clone();
        invalid_weight[6..8].copy_from_slice(&3_u16.to_le_bytes());
        assert!(decode_order2_smoothed_artifact(&invalid_weight, u64::MAX).is_err());
        let mut corrupt = artifact;
        corrupt[24] ^= 1;
        assert!(decode_order2_smoothed_artifact(&corrupt, u64::MAX).is_err());
    }
}
