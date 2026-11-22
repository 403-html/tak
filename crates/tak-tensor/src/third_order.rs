use tak_context::Order3Model;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKO3";

/// Encodes the complete adaptive order-3 research control artifact.
#[must_use]
pub fn encode_order3_artifact(input: &[u8]) -> Vec<u8> {
    let payload = tak_entropy::encode(input, Order3Model::default());
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes the complete adaptive order-3 research control artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_order3_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..8] != [0, 0, 0, 0] {
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
    let output = tak_entropy::decode(&artifact[HEADER_LEN..], output_len, Order3Model::default())
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{decode_order3_artifact, encode_order3_artifact};

    #[test]
    fn artifact_round_trips_and_enforces_limits() {
        for input in [b"".as_slice(), b"a", b"ab", b"abc"] {
            let artifact = encode_order3_artifact(input);
            assert_eq!(
                decode_order3_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
        }

        let input = b"abcXabcYabcXabcY".repeat(100);
        let artifact = encode_order3_artifact(&input);
        assert_eq!(
            decode_order3_artifact(&artifact, input.len() as u64).unwrap(),
            input
        );
        assert!(decode_order3_artifact(&artifact, 10).is_err());
        assert!(decode_order3_artifact(&artifact[..artifact.len() - 1], u64::MAX).is_err());
        let mut invalid_version = artifact.clone();
        invalid_version[4] = 1;
        assert!(decode_order3_artifact(&invalid_version, u64::MAX).is_err());
        let mut corrupt = artifact;
        corrupt[24] ^= 1;
        assert!(decode_order3_artifact(&corrupt, u64::MAX).is_err());
    }
}
