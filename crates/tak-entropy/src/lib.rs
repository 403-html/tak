//! A deterministic, bit-oriented arithmetic coder.

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
        let total = u64::from(model.total());
        let width = high - low + 1;
        let scaled = ((code - low + 1) * total - 1) / width;
        let scaled = u32::try_from(scaled).map_err(|_| DecodeError::InvalidInterval)?;
        if scaled >= model.total() {
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
    use super::{decode, encode};
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
}
