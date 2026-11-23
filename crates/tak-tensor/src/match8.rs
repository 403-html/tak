use std::io::{self, Read, Seek, SeekFrom, Write};

use tak_context::Match8Model;

use crate::{Error, HEADER_LEN, StreamArtifactError, StreamArtifactStats, crc32, read_u64};

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

/// Encodes a byte-identical `TKM8` artifact with bounded I/O buffers.
///
/// # Errors
///
/// Returns an error if the output is nonempty, I/O fails, or a length exceeds
/// the format limit.
pub fn encode_match8_stream(
    input: impl Read,
    mut output: impl Write + Seek,
) -> Result<StreamArtifactStats, StreamArtifactError> {
    if output.seek(SeekFrom::End(0))? != 0 {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&[0_u8; HEADER_LEN])?;
    let mut tracked_input = CrcReader::new(input);
    let stats = tak_entropy::encode_stream(&mut tracked_input, &mut output, Match8Model::new())?;
    if stats.input_bytes != tracked_input.bytes {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
    let artifact_end = 28_u64
        .checked_add(stats.output_bytes)
        .ok_or(StreamArtifactError::Artifact(Error::InvalidArtifact))?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(MAGIC)?;
    output.write_all(&[0; 4])?;
    output.write_all(&tracked_input.bytes.to_le_bytes())?;
    output.write_all(&stats.output_bytes.to_le_bytes())?;
    output.write_all(&tracked_input.crc.finish().to_le_bytes())?;
    output.seek(SeekFrom::Start(artifact_end))?;
    output.flush()?;
    Ok(StreamArtifactStats {
        uncompressed_bytes: tracked_input.bytes,
        payload_bytes: stats.output_bytes,
    })
}

/// Decodes a `TKM8` artifact with bounded I/O buffers.
///
/// # Errors
///
/// Returns an error for invalid fields, extents, output limits, payloads,
/// checksums, or I/O failures.
pub fn decode_match8_stream(
    mut artifact: impl Read + Seek,
    output: impl Write,
    max_output: u64,
) -> Result<StreamArtifactStats, StreamArtifactError> {
    let artifact_len = artifact.seek(SeekFrom::End(0))?;
    artifact.seek(SeekFrom::Start(0))?;
    let mut header = [0_u8; HEADER_LEN];
    artifact
        .read_exact(&mut header)
        .map_err(|error| match error.kind() {
            io::ErrorKind::UnexpectedEof => StreamArtifactError::Artifact(Error::InvalidArtifact),
            _ => StreamArtifactError::Io(error),
        })?;
    if &header[..4] != MAGIC || header[4..8] != [0; 4] {
        return Err(StreamArtifactError::Artifact(Error::InvalidArtifact));
    }
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
    tak_entropy::decode_stream(
        artifact.take(payload_len),
        output_len,
        &mut tracked_output,
        Match8Model::new(),
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
struct Crc32(u32);

impl Crc32 {
    const fn new() -> Self {
        Self(u32::MAX)
    }

    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u32::from(byte);
            for _ in 0..8 {
                let mask = 0_u32.wrapping_sub(self.0 & 1);
                self.0 = (self.0 >> 1) ^ (0xedb8_8320 & mask);
            }
        }
    }

    const fn finish(self) -> u32 {
        !self.0
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
    use std::io::Cursor;

    use super::{
        decode_match8_artifact, decode_match8_stream, encode_match8_artifact, encode_match8_stream,
    };

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

    #[test]
    fn streaming_artifact_is_byte_identical() {
        for input in [
            Vec::new(),
            b"short".to_vec(),
            b"abcdefghXabcdefghXabcdefghY".repeat(2_000),
        ] {
            let buffered = encode_match8_artifact(&input);
            let mut streamed = Cursor::new(Vec::new());
            encode_match8_stream(Cursor::new(&input), &mut streamed).unwrap();
            assert_eq!(streamed.into_inner(), buffered);
            let mut restored = Vec::new();
            decode_match8_stream(Cursor::new(buffered), &mut restored, input.len() as u64).unwrap();
            assert_eq!(restored, input);
        }
    }
}
