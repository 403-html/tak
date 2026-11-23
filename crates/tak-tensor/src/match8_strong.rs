use std::num::NonZeroU32;

use tak_context::Match8Model;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKH8";

fn model() -> Match8Model {
    Match8Model::with_bonus_divisor(NonZeroU32::MIN)
}

/// Encodes a complete RFC 0010 high-confidence match artifact.
#[must_use]
pub fn encode_match8_strong_artifact(input: &[u8]) -> Vec<u8> {
    let payload = tak_entropy::encode(input, model());
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&[0; 4]);
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(&payload);
    output
}

/// Decodes a complete RFC 0010 high-confidence match artifact.
///
/// # Errors
///
/// Returns an error for invalid fields, excessive output, an invalid payload,
/// or a checksum mismatch.
pub fn decode_match8_strong_artifact(artifact: &[u8], max_output: u64) -> Result<Vec<u8>, Error> {
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
    let output = tak_entropy::decode(&artifact[HEADER_LEN..], output_len, model())
        .map_err(|_| Error::InvalidPayload)?;
    if crc32(&output) != expected_crc {
        return Err(Error::ChecksumMismatch);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{decode_match8_strong_artifact, encode_match8_strong_artifact};

    #[test]
    fn strong_artifact_round_trips() {
        let input = b"abcdefghXabcdefghXabcdefghY".repeat(1_000);
        let artifact = encode_match8_strong_artifact(&input);
        assert_eq!(
            decode_match8_strong_artifact(&artifact, input.len() as u64).unwrap(),
            input
        );
    }
}
