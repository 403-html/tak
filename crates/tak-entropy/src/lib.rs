//! A deterministic, bit-oriented arithmetic coder.

use std::io::{self, BufReader, Read, Write};

use tak_context::ByteModel;

const MAX: u64 = u32::MAX as u64;
const HALF: u64 = 0x8000_0000;
const QUARTER: u64 = 0x4000_0000;
const THREE_QUARTERS: u64 = 0xc000_0000;

/// Errors reported by the arithmetic decoder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The encoded interval became invalid.
    InvalidInterval,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid arithmetic-coded payload")
    }
}

impl std::error::Error for DecodeError {}

/// Byte counts produced by streaming arithmetic encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeStats {
    /// Uncompressed bytes consumed from the reader.
    pub input_bytes: u64,
    /// Compressed payload bytes written to the writer.
    pub output_bytes: u64,
}

/// Errors reported by streaming arithmetic decoding.
#[derive(Debug)]
pub enum StreamDecodeError {
    /// The encoded interval became invalid.
    InvalidInterval,
    /// Reading or writing failed.
    Io(io::Error),
}

impl std::fmt::Display for StreamDecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInterval => formatter.write_str("invalid arithmetic-coded payload"),
            Self::Io(error) => write!(formatter, "stream I/O failed: {error}"),
        }
    }
}

impl std::error::Error for StreamDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidInterval => None,
            Self::Io(error) => Some(error),
        }
    }
}

impl From<io::Error> for StreamDecodeError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Encodes all bytes with a deterministic adaptive model.
pub fn encode(input: &[u8], mut model: impl ByteModel) -> Vec<u8> {
    let mut writer = BitWriter::default();
    let mut low = 0_u64;
    let mut high = MAX;
    let mut pending = 0_usize;

    for &symbol in input {
        let (cum_low, cum_high, total) = model.range(symbol);
        let width = high - low + 1;
        high = low + width * u64::from(cum_high) / u64::from(total) - 1;
        low += width * u64::from(cum_low) / u64::from(total);

        loop {
            if high < HALF {
                writer.bit_with_pending(false, &mut pending);
            } else if low >= HALF {
                writer.bit_with_pending(true, &mut pending);
                low -= HALF;
                high -= HALF;
            } else if low >= QUARTER && high < THREE_QUARTERS {
                pending += 1;
                low -= QUARTER;
                high -= QUARTER;
            } else {
                break;
            }
            low <<= 1;
            high = (high << 1) | 1;
        }
        model.update(symbol);
    }

    pending += 1;
    writer.bit_with_pending(low >= QUARTER, &mut pending);
    writer.finish()
}

/// Encodes bytes from `input` to `output` without retaining the full payload.
///
/// # Errors
///
/// Returns an error if reading or writing fails or a byte count exceeds
/// `u64::MAX`.
pub fn encode_stream(
    mut input: impl Read,
    output: impl Write,
    mut model: impl ByteModel,
) -> io::Result<EncodeStats> {
    let mut writer = StreamBitWriter::new(output);
    let mut low = 0_u64;
    let mut high = MAX;
    let mut pending = 0_usize;
    let mut input_bytes = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024];

    loop {
        let read_count = match input.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let read =
            u64::try_from(read_count).map_err(|_| io::Error::other("read length exceeds u64"))?;
        input_bytes = input_bytes
            .checked_add(read)
            .ok_or_else(|| io::Error::other("input length exceeds u64"))?;
        for &symbol in &buffer[..read_count] {
            let (cum_low, cum_high, total) = model.range(symbol);
            let width = high - low + 1;
            high = low + width * u64::from(cum_high) / u64::from(total) - 1;
            low += width * u64::from(cum_low) / u64::from(total);

            loop {
                if high < HALF {
                    writer.bit_with_pending(false, &mut pending)?;
                } else if low >= HALF {
                    writer.bit_with_pending(true, &mut pending)?;
                    low -= HALF;
                    high -= HALF;
                } else if low >= QUARTER && high < THREE_QUARTERS {
                    pending += 1;
                    low -= QUARTER;
                    high -= QUARTER;
                } else {
                    break;
                }
                low <<= 1;
                high = (high << 1) | 1;
            }
            model.update(symbol);
        }
    }

    pending += 1;
    writer.bit_with_pending(low >= QUARTER, &mut pending)?;
    let output_bytes = writer.finish()?;
    Ok(EncodeStats {
        input_bytes,
        output_bytes,
    })
}

/// Decodes exactly `output_len` bytes with the matching adaptive model.
///
/// # Errors
///
/// Returns [`DecodeError::InvalidInterval`] if the payload produces an invalid
/// arithmetic-coder state.
pub fn decode(
    payload: &[u8],
    output_len: usize,
    mut model: impl ByteModel,
) -> Result<Vec<u8>, DecodeError> {
    if output_len == 0 {
        return Ok(Vec::new());
    }

    let mut reader = BitReader::new(payload);
    let mut low = 0_u64;
    let mut high = MAX;
    let mut code = 0_u64;
    for _ in 0..32 {
        code = (code << 1) | u64::from(reader.bit());
    }

    let mut output = Vec::with_capacity(output_len);
    for _ in 0..output_len {
        if code < low || code > high {
            return Err(DecodeError::InvalidInterval);
        }
        let model_total = model.total();
        let total = u64::from(model_total);
        let width = high - low + 1;
        let scaled = ((code - low + 1) * total - 1) / width;
        let scaled = u32::try_from(scaled).map_err(|_| DecodeError::InvalidInterval)?;
        if scaled >= model_total {
            return Err(DecodeError::InvalidInterval);
        }
        let (symbol, cum_low, cum_high, actual_total) = model.symbol(scaled);
        high = low + width * u64::from(cum_high) / u64::from(actual_total) - 1;
        low += width * u64::from(cum_low) / u64::from(actual_total);

        loop {
            if high < HALF {
                // No offset is necessary.
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
            code = ((code << 1) & MAX) | u64::from(reader.bit());
        }
        output.push(symbol);
        model.update(symbol);
    }
    Ok(output)
}

/// Decodes exactly `output_len` bytes from a streaming payload.
///
/// # Errors
///
/// Returns an error for an invalid arithmetic interval or an I/O failure.
pub fn decode_stream(
    input: impl Read,
    output_len: u64,
    mut output: impl Write,
    mut model: impl ByteModel,
) -> Result<(), StreamDecodeError> {
    if output_len == 0 {
        return Ok(());
    }

    let mut reader = StreamBitReader::new(input);
    let mut low = 0_u64;
    let mut high = MAX;
    let mut code = 0_u64;
    for _ in 0..32 {
        code = (code << 1) | u64::from(reader.bit()?);
    }

    let mut output_buffer = Vec::with_capacity(64 * 1024);
    for _ in 0..output_len {
        if code < low || code > high {
            return Err(StreamDecodeError::InvalidInterval);
        }
        let model_total = model.total();
        let total = u64::from(model_total);
        let width = high - low + 1;
        let scaled = ((code - low + 1) * total - 1) / width;
        let scaled = u32::try_from(scaled).map_err(|_| StreamDecodeError::InvalidInterval)?;
        if scaled >= model_total {
            return Err(StreamDecodeError::InvalidInterval);
        }
        let (symbol, cum_low, cum_high, actual_total) = model.symbol(scaled);
        high = low + width * u64::from(cum_high) / u64::from(actual_total) - 1;
        low += width * u64::from(cum_low) / u64::from(actual_total);

        loop {
            if high < HALF {
                // No offset is necessary.
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
            code = ((code << 1) & MAX) | u64::from(reader.bit()?);
        }
        output_buffer.push(symbol);
        if output_buffer.len() == output_buffer.capacity() {
            output.write_all(&output_buffer)?;
            output_buffer.clear();
        }
        model.update(symbol);
    }
    output.write_all(&output_buffer)?;
    output.flush()?;
    Ok(())
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

struct BitReader<'a> {
    bytes: &'a [u8],
    bit_index: usize,
}

struct StreamBitWriter<W> {
    output: W,
    bytes: Vec<u8>,
    current: u8,
    used: u8,
    written: u64,
}

impl<W: Write> StreamBitWriter<W> {
    fn new(output: W) -> Self {
        Self {
            output,
            bytes: Vec::with_capacity(64 * 1024),
            current: 0,
            used: 0,
            written: 0,
        }
    }

    fn bit(&mut self, bit: bool) -> io::Result<()> {
        self.current = (self.current << 1) | u8::from(bit);
        self.used += 1;
        if self.used == 8 {
            self.bytes.push(self.current);
            self.current = 0;
            self.used = 0;
            if self.bytes.len() == self.bytes.capacity() {
                self.flush_bytes()?;
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

    fn flush_bytes(&mut self) -> io::Result<()> {
        self.output.write_all(&self.bytes)?;
        self.written = self
            .written
            .checked_add(u64::try_from(self.bytes.len()).expect("buffer length fits u64"))
            .ok_or_else(|| io::Error::other("payload length exceeds u64"))?;
        self.bytes.clear();
        Ok(())
    }

    fn finish(mut self) -> io::Result<u64> {
        if self.used != 0 {
            self.current <<= 8 - self.used;
            self.bytes.push(self.current);
        }
        self.flush_bytes()?;
        self.output.flush()?;
        Ok(self.written)
    }
}

struct StreamBitReader<R> {
    input: BufReader<R>,
    current: u8,
    remaining: u8,
}

impl<R: Read> StreamBitReader<R> {
    fn new(input: R) -> Self {
        Self {
            input: BufReader::with_capacity(64 * 1024, input),
            current: 0,
            remaining: 0,
        }
    }

    fn bit(&mut self) -> io::Result<u8> {
        if self.remaining == 0 {
            let mut byte = [0_u8; 1];
            loop {
                match self.input.read(&mut byte) {
                    Ok(0) => {
                        self.current = 0;
                        break;
                    }
                    Ok(_) => {
                        self.current = byte[0];
                        break;
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                    Err(error) => return Err(error),
                }
            }
            self.remaining = 8;
        }
        let bit = (self.current >> (self.remaining - 1)) & 1;
        self.remaining -= 1;
        Ok(bit)
    }
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{decode, decode_stream, encode, encode_stream};
    use tak_context::{Order0Model, Order1Model, Order2Model};

    #[test]
    fn round_trips_both_models() {
        let inputs = [
            Vec::new(),
            b"a".to_vec(),
            b"abracadabra abracadabra".to_vec(),
            (0_u8..=255).collect(),
            vec![0; 100_000],
        ];
        for input in inputs {
            let encoded = encode(&input, Order0Model::default());
            assert_eq!(
                decode(&encoded, input.len(), Order0Model::default()).unwrap(),
                input
            );
            let encoded = encode(&input, Order1Model::default());
            assert_eq!(
                decode(&encoded, input.len(), Order1Model::default()).unwrap(),
                input
            );
            let encoded = encode(&input, Order2Model::default());
            assert_eq!(
                decode(&encoded, input.len(), Order2Model::default()).unwrap(),
                input
            );
        }
    }

    #[test]
    fn streaming_coder_is_byte_identical() {
        let inputs = [
            Vec::new(),
            b"a".to_vec(),
            b"abracadabra abracadabra".repeat(100),
            (0_u8..=255).cycle().take(40_000).collect(),
        ];
        for input in inputs {
            let buffered = encode(&input, Order2Model::default());
            let mut streamed = Vec::new();
            let stats =
                encode_stream(Cursor::new(&input), &mut streamed, Order2Model::default()).unwrap();
            assert_eq!(streamed, buffered);
            assert_eq!(stats.input_bytes, input.len() as u64);
            assert_eq!(stats.output_bytes, buffered.len() as u64);

            let mut restored = Vec::new();
            decode_stream(
                Cursor::new(&streamed),
                input.len() as u64,
                &mut restored,
                Order2Model::default(),
            )
            .unwrap();
            assert_eq!(restored, input);
        }
    }
}
