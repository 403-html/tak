//! Experimental TAK-C framing and byte codecs.
//!
//! Version zero is intentionally unstable. All multi-byte integers are
//! little-endian. Decoders must enforce an output limit supplied by callers.

use tak_context::{Order0Model, Order1Model};

const MAGIC: &[u8; 4] = b"TAKC";
const VERSION: u8 = 0;
const HEADER_LEN: usize = 28;
const FLAG_CRC32: u16 = 1;

/// An implemented compression method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Method {
    /// The payload contains the original bytes.
    Raw = 0,
    /// Adaptive order-0 byte probabilities and arithmetic coding.
    Order0 = 1,
    /// Adaptive order-1 byte probabilities and arithmetic coding.
    Order1 = 2,
}

impl Method {
    /// Returns the stable CLI name used by the experiment.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Order0 => "order-0",
            Self::Order1 => "order-1",
        }
    }

    /// Returns every method in benchmark order.
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [Self::Raw, Self::Order0, Self::Order1]
    }
}

impl TryFrom<u8> for Method {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Raw),
            1 => Ok(Self::Order0),
            2 => Ok(Self::Order1),
            _ => Err(Error::UnsupportedMethod(value)),
        }
    }
}

/// Failures reported while parsing or decoding a stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The stream is shorter than the fixed header.
    TruncatedHeader,
    /// The four-byte stream identifier is not `TAKC`.
    InvalidMagic,
    /// The version is not supported by this experimental decoder.
    UnsupportedVersion(u8),
    /// The method identifier is unknown.
    UnsupportedMethod(u8),
    /// A required or reserved flag is invalid.
    InvalidFlags(u16),
    /// A declared length cannot be represented or exceeds the input.
    InvalidLength,
    /// The requested output would exceed the caller's limit.
    OutputLimitExceeded { declared: u64, limit: u64 },
    /// The raw method has inconsistent lengths.
    InvalidRawPayload,
    /// Entropy decoding failed.
    InvalidPayload,
    /// The decoded data does not match the stored checksum.
    ChecksumMismatch { expected: u32, actual: u32 },
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TruncatedHeader => formatter.write_str("truncated TAK-C header"),
            Self::InvalidMagic => formatter.write_str("invalid TAK-C magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported TAK-C version {version}")
            }
            Self::UnsupportedMethod(method) => {
                write!(formatter, "unsupported compression method {method}")
            }
            Self::InvalidFlags(flags) => write!(formatter, "invalid TAK-C flags 0x{flags:04x}"),
            Self::InvalidLength => formatter.write_str("invalid TAK-C length"),
            Self::OutputLimitExceeded { declared, limit } => {
                write!(
                    formatter,
                    "declared output size {declared} exceeds limit {limit}"
                )
            }
            Self::InvalidRawPayload => {
                formatter.write_str("raw payload length does not match output length")
            }
            Self::InvalidPayload => formatter.write_str("invalid compressed payload"),
            Self::ChecksumMismatch { expected, actual } => {
                write!(
                    formatter,
                    "checksum mismatch: expected {expected:08x}, got {actual:08x}"
                )
            }
        }
    }
}

impl std::error::Error for Error {}

/// Basic information available without decoding the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    /// Compression method stored in the frame.
    pub method: Method,
    /// Number of bytes produced by a valid decoder.
    pub uncompressed_len: u64,
    /// Number of encoded payload bytes.
    pub payload_len: u64,
    /// CRC-32/ISO-HDLC of the uncompressed bytes.
    pub checksum: u32,
}

/// Encodes one frame with the selected method.
#[must_use]
pub fn encode(input: &[u8], method: Method) -> Vec<u8> {
    let payload = match method {
        Method::Raw => input.to_vec(),
        Method::Order0 => tak_entropy::encode(input, Order0Model::default()),
        Method::Order1 => tak_entropy::encode(input, Order1Model::default()),
    };
    frame(input, method, &payload)
}

/// Encodes all implemented methods and returns the smallest complete frame.
/// Ties prefer the method with the smaller identifier.
#[must_use]
pub fn encode_auto(input: &[u8]) -> Vec<u8> {
    let mut smallest = encode(input, Method::Raw);
    for method in [Method::Order0, Method::Order1] {
        let candidate = encode(input, method);
        if candidate.len() < smallest.len() {
            smallest = candidate;
        }
    }
    smallest
}

/// Parses the fixed frame header and validates its declared payload extent.
///
/// # Errors
///
/// Returns an error if a header field is invalid, unsupported, or inconsistent
/// with the physical frame length.
pub fn inspect(frame: &[u8]) -> Result<FrameInfo, Error> {
    if frame.len() < HEADER_LEN {
        return Err(Error::TruncatedHeader);
    }
    if &frame[..4] != MAGIC {
        return Err(Error::InvalidMagic);
    }
    if frame[4] != VERSION {
        return Err(Error::UnsupportedVersion(frame[4]));
    }
    let method = Method::try_from(frame[5])?;
    let flags = u16::from_le_bytes([frame[6], frame[7]]);
    if flags != FLAG_CRC32 {
        return Err(Error::InvalidFlags(flags));
    }
    let uncompressed_len = read_u64(&frame[8..16]);
    let payload_len = read_u64(&frame[16..24]);
    let checksum = u32::from_le_bytes([frame[24], frame[25], frame[26], frame[27]]);
    let payload_len_usize = usize::try_from(payload_len).map_err(|_| Error::InvalidLength)?;
    let expected_len = HEADER_LEN
        .checked_add(payload_len_usize)
        .ok_or(Error::InvalidLength)?;
    if frame.len() != expected_len {
        return Err(Error::InvalidLength);
    }
    Ok(FrameInfo {
        method,
        uncompressed_len,
        payload_len,
        checksum,
    })
}

/// Decodes one frame while enforcing `max_output_len` before allocation.
///
/// # Errors
///
/// Returns an error if the frame is invalid, exceeds the output limit, cannot
/// be decoded, or fails its checksum verification.
pub fn decode(frame: &[u8], max_output_len: u64) -> Result<Vec<u8>, Error> {
    let info = inspect(frame)?;
    if info.uncompressed_len > max_output_len {
        return Err(Error::OutputLimitExceeded {
            declared: info.uncompressed_len,
            limit: max_output_len,
        });
    }
    let output_len = usize::try_from(info.uncompressed_len).map_err(|_| Error::InvalidLength)?;
    let payload = &frame[HEADER_LEN..];
    let output = match info.method {
        Method::Raw => {
            if payload.len() != output_len {
                return Err(Error::InvalidRawPayload);
            }
            payload.to_vec()
        }
        Method::Order0 => tak_entropy::decode(payload, output_len, Order0Model::default())
            .map_err(|_| Error::InvalidPayload)?,
        Method::Order1 => tak_entropy::decode(payload, output_len, Order1Model::default())
            .map_err(|_| Error::InvalidPayload)?,
    };
    let actual = crc32(&output);
    if actual != info.checksum {
        return Err(Error::ChecksumMismatch {
            expected: info.checksum,
            actual,
        });
    }
    Ok(output)
}

fn frame(input: &[u8], method: Method, payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::with_capacity(HEADER_LEN + payload.len());
    output.extend_from_slice(MAGIC);
    output.push(VERSION);
    output.push(method as u8);
    output.extend_from_slice(&FLAG_CRC32.to_le_bytes());
    output.extend_from_slice(&(input.len() as u64).to_le_bytes());
    output.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    output.extend_from_slice(&crc32(input).to_le_bytes());
    output.extend_from_slice(payload);
    output
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
    use super::{Error, HEADER_LEN, Method, decode, encode, encode_auto, inspect};

    fn data_sets() -> Vec<Vec<u8>> {
        let mut pseudo_random = Vec::with_capacity(100_000);
        let mut state = 0x1234_5678_u32;
        for _ in 0..100_000 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            pseudo_random.push((state >> 24) as u8);
        }
        vec![
            Vec::new(),
            b"x".to_vec(),
            b"the quick brown fox jumps over the lazy dog\n".repeat(1_000),
            (0_u8..=255).cycle().take(70_000).collect(),
            pseudo_random,
        ]
    }

    #[test]
    fn every_method_round_trips() {
        for data in data_sets() {
            for method in Method::all() {
                let frame = encode(&data, method);
                assert_eq!(decode(&frame, data.len() as u64).unwrap(), data);
                assert_eq!(inspect(&frame).unwrap().method, method);
            }
        }
    }

    #[test]
    fn automatic_mode_returns_a_valid_smallest_frame() {
        let data = b"abababababababababababababababab".repeat(100);
        let frame = encode_auto(&data);
        let smallest = Method::all()
            .into_iter()
            .map(|method| encode(&data, method).len())
            .min()
            .unwrap();
        assert_eq!(frame.len(), smallest);
        assert_eq!(decode(&frame, data.len() as u64).unwrap(), data);
    }

    #[test]
    fn rejects_corruption_and_truncation() {
        let original = b"integrity matters".repeat(100);
        let mut frame = encode(&original, Method::Order1);
        frame[HEADER_LEN] ^= 0x40;
        assert!(matches!(
            decode(&frame, u64::MAX),
            Err(Error::ChecksumMismatch { .. } | Error::InvalidPayload)
        ));
        frame.pop();
        assert_eq!(decode(&frame, u64::MAX), Err(Error::InvalidLength));
    }

    #[test]
    fn enforces_output_limit_before_decoding() {
        let frame = encode(b"12345", Method::Raw);
        assert_eq!(
            decode(&frame, 4),
            Err(Error::OutputLimitExceeded {
                declared: 5,
                limit: 4,
            })
        );
    }

    #[test]
    fn arbitrary_short_inputs_do_not_panic() {
        for length in 0..128 {
            let bytes: Vec<u8> = (0..length)
                .map(|index| u8::try_from(index).unwrap().wrapping_mul(73))
                .collect();
            let result = std::panic::catch_unwind(|| decode(&bytes, 1_024));
            assert!(result.is_ok());
        }
    }

    #[test]
    fn single_bit_corruption_does_not_panic() {
        let original = b"decoder input is untrusted".repeat(20);
        let valid = encode(&original, Method::Order1);
        for index in 0..valid.len() {
            for bit in 0..8 {
                let mut corrupted = valid.clone();
                corrupted[index] ^= 1 << bit;
                let result = std::panic::catch_unwind(|| decode(&corrupted, 1 << 20));
                assert!(
                    result.is_ok(),
                    "panic after changing byte {index}, bit {bit}"
                );
            }
        }
    }
}
