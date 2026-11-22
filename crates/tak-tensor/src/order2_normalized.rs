use std::num::NonZeroU16;

use tak_context::Order2NormalizedModel;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKN2";
const STRENGTHS: [u16; 6] = [1, 4, 16, 64, 256, 1024];

/// Encodes a complete finite-strength order-2 research artifact.
///
/// # Panics
///
/// Panics if `prior_strength` is not an RFC 0006 screening strength.
#[must_use]
pub fn encode_order2_normalized_artifact(input: &[u8], prior_strength: NonZeroU16) -> Vec<u8> {
    assert!(STRENGTHS.contains(&prior_strength.get()));
    let payload = tak_entropy::encode(input, Order2NormalizedModel::new(prior_strength));
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0, 0]);
    output.extend_from_slice(&prior_strength.get().to_le_bytes());
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete finite-strength order-2 research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_order2_normalized_artifact(
    artifact: &[u8],
    max_output: u64,
) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..6] != [0, 0] {
        return Err(Error::InvalidArtifact);
    }
    let strength = u16::from_le_bytes([artifact[6], artifact[7]]);
    if !STRENGTHS.contains(&strength) {
        return Err(Error::InvalidArtifact);
    }
    let prior_strength = NonZeroU16::new(strength).ok_or(Error::InvalidArtifact)?;
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
    let output = tak_entropy::decode(
        &artifact[HEADER_LEN..],
        output_len,
        Order2NormalizedModel::new(prior_strength),
    )
    .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;

    use super::{decode_order2_normalized_artifact, encode_order2_normalized_artifact};

    #[test]
    fn every_strength_round_trips() {
        let input = b"abXabYabXabY".repeat(100);
        for strength in [1, 4, 16, 64, 256, 1024] {
            let artifact =
                encode_order2_normalized_artifact(&input, NonZeroU16::new(strength).unwrap());
            assert_eq!(
                decode_order2_normalized_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }
    }

    #[test]
    fn rejects_invalid_fields_corruption_and_output_excess() {
        let strength = NonZeroU16::new(16).unwrap();
        let input = b"finite prior".repeat(20);
        let artifact = encode_order2_normalized_artifact(&input, strength);
        assert!(decode_order2_normalized_artifact(&artifact, 1).is_err());
        assert!(
            decode_order2_normalized_artifact(&artifact[..artifact.len() - 1], u64::MAX).is_err()
        );
        let mut invalid_strength = artifact.clone();
        invalid_strength[6..8].copy_from_slice(&2_u16.to_le_bytes());
        assert!(decode_order2_normalized_artifact(&invalid_strength, u64::MAX).is_err());
        let mut corrupt = artifact;
        corrupt[24] ^= 1;
        assert!(decode_order2_normalized_artifact(&corrupt, u64::MAX).is_err());
    }
}
