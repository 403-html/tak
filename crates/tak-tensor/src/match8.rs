use tak_context::Match8Model;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKM8";

/// Encodes a complete RFC 0009 bounded-match research artifact.
#[must_use]
pub fn encode_match8_artifact(input: &[u8]) -> Vec<u8> {
    let payload = tak_entropy::encode(input, Match8Model::new());
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete RFC 0009 bounded-match research artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_match8_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
    if artifact.len() < HEADER_LEN || &artifact[..4] != MAGIC || artifact[4..8] != [0; 4] {
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
    let output = tak_entropy::decode(&artifact[HEADER_LEN..], output_len, Match8Model::new())
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{decode_match8_artifact, encode_match8_artifact};

    #[test]
    fn artifact_round_trips_and_rejects_invalid_input() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXabcdefghXabcdefghY".repeat(1_000),
        ] {
            let artifact = encode_match8_artifact(&input);
            assert_eq!(
                decode_match8_artifact(&artifact, input.len() as u64).unwrap(),
                input
            );
            assert!(decode_match8_artifact(&artifact, 1).is_err() || input.len() <= 1);

            if !artifact.is_empty() {
                let mut corrupt = artifact.clone();
                corrupt[0] ^= 1;
                assert!(decode_match8_artifact(&corrupt, u64::MAX).is_err());
                assert!(decode_match8_artifact(&artifact[..artifact.len() - 1], u64::MAX).is_err());
            }
        }
    }
}
