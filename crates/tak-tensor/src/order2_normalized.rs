use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    num::NonZeroU16,
};

use tak_context::Order2NormalizedModel;

use crate::{Error, HEADER_LEN, crc32, read_u64};

const MAGIC: &[u8; 4] = b"TKN2";
const STRENGTHS: [u16; 6] = [1, 4, 16, 64, 256, 1024];

/// Byte counts produced by streaming artifact encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamArtifactStats {
    /// Uncompressed bytes consumed or produced.
    pub uncompressed_bytes: u64,
    /// Arithmetic-coded payload bytes written after the header.
    pub payload_bytes: u64,
}

/// Errors produced by streaming `TKN2` operations.
#[derive(Debug)]
pub enum StreamArtifactError {
    /// The artifact is invalid or violates a caller limit.
    Artifact(Error),
    /// Reading, writing, or seeking failed.
    Io(io::Error),
}

impl std::fmt::Display for StreamArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(error) => error.fmt(formatter),
            Self::Io(error) => write!(formatter, "stream I/O failed: {error}"),
        }
    }
}

impl std::error::Error for StreamArtifactError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Artifact(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<io::Error> for StreamArtifactError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

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

/// Encodes a byte-identical `TKN2` artifact with bounded I/O buffers.
///
/// The output must be empty and positioned at offset zero. The function seeks
/// back to patch the final lengths and checksum.
///
/// # Errors
///
/// Returns an error for an unsupported strength, a nonempty output, an I/O
/// failure, or a length overflow.
pub fn encode_order2_normalized_stream(
    input: impl Read,
    mut output: impl Write + Seek,
    prior_strength: NonZeroU16,
) -> Result<StreamArtifactStats, StreamArtifactError> {
    if !STRENGTHS.contains(&prior_strength.get()) {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    if output.seek(SeekFrom::End(0))? != 0 {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&[0_u8; HEADER_LEN])?;

    let mut tracked_input = CrcReader::new(input);
    let stats = tak_entropy::encode_stream(
        &mut tracked_input,
        &mut output,
        Order2NormalizedModel::new(prior_strength),
    )?;
    let input_bytes = tracked_input.bytes;
    if stats.input_bytes != input_bytes {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    let payload_bytes = stats.output_bytes;
    let artifact_end = 28_u64
        .checked_add(payload_bytes)
        .ok_or(StreamArtifactError::Artifact(Error::InvalidArtifact))?;

    output.seek(SeekFrom::Start(0))?;
    output.write_all(MAGIC)?;
    output.write_all(&[0, 0])?;
    output.write_all(&prior_strength.get().to_le_bytes())?;
    output.write_all(&input_bytes.to_le_bytes())?;
    output.write_all(&payload_bytes.to_le_bytes())?;
    output.write_all(&tracked_input.crc.finish().to_le_bytes())?;
    output.seek(SeekFrom::Start(artifact_end))?;
    output.flush()?;
    Ok(StreamArtifactStats {
        uncompressed_bytes: input_bytes,
        payload_bytes,
    })
}

/// Decodes a `TKN2` artifact with bounded I/O buffers.
///
/// # Errors
///
/// Returns an error for invalid fields or extents, excessive output, an
/// invalid arithmetic payload, checksum failure, or an I/O failure.
pub fn decode_order2_normalized_stream(
    mut artifact: impl Read + Seek,
    output: impl Write,
    max_output: u64,
) -> Result<StreamArtifactStats, StreamArtifactError> {
    let artifact_len = artifact.seek(SeekFrom::End(0))?;
    artifact.seek(SeekFrom::Start(0))?;
    let mut header = [0_u8; HEADER_LEN];
    if let Err(error) = artifact.read_exact(&mut header) {
        return if error.kind() == io::ErrorKind::UnexpectedEof {
            Err(StreamArtifactError::Artifact(Error::InvalidArtifact))
        } else {
            Err(StreamArtifactError::Io(error))
        };
    }
    if &header[..4] != MAGIC || header[4..6] != [0, 0] {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    let strength = u16::from_le_bytes([header[6], header[7]]);
    if !STRENGTHS.contains(&strength) {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    let prior_strength =
        NonZeroU16::new(strength).ok_or(StreamArtifactError::Artifact(Error::InvalidArtifact))?;
    let output_len = read_u64(&header[8..16]);
    if output_len > max_output {
        return Err(StreamArtifactError::Artifact(Error::OutputLimitExceeded));
    }
    let payload_len = read_u64(&header[16..24]);
    let expected_len = 28_u64
        .checked_add(payload_len)
        .ok_or(StreamArtifactError::Artifact(Error::InvalidArtifact))?;
    if artifact_len != expected_len {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    let expected_crc = u32::from_le_bytes([header[24], header[25], header[26], header[27]]);

    let mut tracked_output = CrcWriter::new(output);
    let mut payload = artifact.take(payload_len);
    tak_entropy::decode_stream(
        &mut payload,
        output_len,
        &mut tracked_output,
        Order2NormalizedModel::new(prior_strength),
    )
    .map_err(|error| match error {
        tak_entropy::StreamDecodeError::InvalidInterval => {
            StreamArtifactError::Artifact(Error::InvalidPayload)
        }
        tak_entropy::StreamDecodeError::Io(error) => StreamArtifactError::Io(error),
    })?;
    if tracked_output.bytes != output_len {
        return Err(StreamArtifactError::Artifact(Error::InvalidPayload));
    }
    if tracked_output.crc.finish() != expected_crc {
        return Err(StreamArtifactError::Artifact(Error::ChecksumMismatch));
    }
    Ok(StreamArtifactStats {
        uncompressed_bytes: output_len,
        payload_bytes: payload_len,
    })
}

#[derive(Clone, Copy)]
struct Crc32 {
    state: u32,
}

impl Crc32 {
    const fn new() -> Self {
        Self { state: u32::MAX }
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u32::from(byte);
            for _ in 0..8 {
                let mask = 0_u32.wrapping_sub(self.state & 1);
                self.state = (self.state >> 1) ^ (0xedb8_8320 & mask);
            }
        }
    }

    const fn finish(self) -> u32 {
        !self.state
    }
}

struct CrcReader<R> {
    inner: R,
    crc: Crc32,
    bytes: u64,
}

impl<R> CrcReader<R> {
    const fn new(inner: R) -> Self {
        Self {
            inner,
            crc: Crc32::new(),
            bytes: 0,
        }
    }
}

impl<R: Read> Read for CrcReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        self.crc.update(&buffer[..read]);
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(read).map_err(|_| io::Error::other("read exceeds u64"))?)
            .ok_or_else(|| io::Error::other("input length exceeds u64"))?;
        Ok(read)
    }
}

struct CrcWriter<W> {
    inner: W,
    crc: Crc32,
    bytes: u64,
}

impl<W> CrcWriter<W> {
    const fn new(inner: W) -> Self {
        Self {
            inner,
            crc: Crc32::new(),
            bytes: 0,
        }
    }
}

impl<W: Write> Write for CrcWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        self.crc.update(&buffer[..written]);
        self.bytes = self
            .bytes
            .checked_add(u64::try_from(written).map_err(|_| io::Error::other("write exceeds u64"))?)
            .ok_or_else(|| io::Error::other("output length exceeds u64"))?;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, num::NonZeroU16};

    use super::{
        decode_order2_normalized_artifact, decode_order2_normalized_stream,
        encode_order2_normalized_artifact, encode_order2_normalized_stream,
    };

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

    #[test]
    fn streaming_artifact_is_byte_identical() {
        for input in [
            Vec::new(),
            b"a".to_vec(),
            b"finite prior streaming".repeat(1_000),
            (0_u8..=255).cycle().take(40_000).collect(),
        ] {
            let strength = NonZeroU16::new(4).unwrap();
            let buffered = encode_order2_normalized_artifact(&input, strength);
            let mut streamed = Cursor::new(Vec::new());
            let stats =
                encode_order2_normalized_stream(Cursor::new(&input), &mut streamed, strength)
                    .unwrap();
            assert_eq!(streamed.into_inner(), buffered);
            assert_eq!(stats.uncompressed_bytes, input.len() as u64);
            assert_eq!(stats.payload_bytes + 28, buffered.len() as u64);

            let mut restored = Vec::new();
            decode_order2_normalized_stream(
                Cursor::new(buffered),
                &mut restored,
                input.len() as u64,
            )
            .unwrap();
            assert_eq!(restored, input);
        }
    }

    #[test]
    fn streaming_decoder_rejects_invalid_extent_limit_and_checksum() {
        let input = b"stream rejection".repeat(100);
        let strength = NonZeroU16::new(4).unwrap();
        let artifact = encode_order2_normalized_artifact(&input, strength);
        let mut output = Vec::new();
        assert!(decode_order2_normalized_stream(Cursor::new(&artifact), &mut output, 1).is_err());
        output.clear();
        assert!(
            decode_order2_normalized_stream(
                Cursor::new(&artifact[..artifact.len() - 1]),
                &mut output,
                u64::MAX,
            )
            .is_err()
        );
        let mut invalid_magic = artifact.clone();
        invalid_magic[0] ^= 1;
        output.clear();
        assert!(
            decode_order2_normalized_stream(Cursor::new(invalid_magic), &mut output, u64::MAX,)
                .is_err()
        );
        let mut unsupported_strength = artifact.clone();
        unsupported_strength[6..8].copy_from_slice(&2_u16.to_le_bytes());
        output.clear();
        assert!(
            decode_order2_normalized_stream(
                Cursor::new(unsupported_strength),
                &mut output,
                u64::MAX,
            )
            .is_err()
        );
        let mut corrupt = artifact;
        corrupt[24] ^= 1;
        output.clear();
        assert!(
            decode_order2_normalized_stream(Cursor::new(corrupt), &mut output, u64::MAX).is_err()
        );
    }
}
