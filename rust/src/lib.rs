use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{error::Error, fmt, io::Cursor};

#[path = "generated/decoders.rs"]
pub mod decoders;
pub mod time;

const TDF_TIME_MASK: u16 = 0xC000;
const TDF_ARRAY_MASK: u16 = 0x3000;
const TDF_ID_MASK: u16 = 0x0FFF;

const TDF_TIME_NONE: u16 = 0x0000;
const TDF_TIME_GLOBAL: u16 = 0x4000;
const TDF_TIME_RELATIVE_U16: u16 = 0x8000;
const TDF_TIME_RELATIVE_S24: u16 = 0xC000;

const TDF_ARRAY_NONE: u16 = 0x0000;
const TDF_ARRAY_TIME: u16 = 0x1000;
const TDF_ARRAY_DIFF: u16 = 0x2000;
const TDF_ARRAY_IDX: u16 = 0x3000;

const TDF_DIFF_16_8: u8 = 1;
const TDF_DIFF_32_8: u8 = 2;
const TDF_DIFF_32_16: u8 = 3;

const TDF_PERIOD_SCALING_BIT: u16 = 0x8000;
const TDF_PERIOD_SCALING_VAL_MASK: u16 = 0x7FFF;
const TDF_PERIOD_SCALING_MULT: u16 = 8192;

pub type TdfDecodeResult<T> = Result<T, TdfDecodeError>;

#[derive(Debug)]
pub enum TdfDecodeError {
    Io(std::io::Error),
    InvalidData(&'static str),
    UnexpectedEof(&'static str),
    MissingAbsoluteTimestamp(&'static str),
    UnknownDiffType(u8),
}

impl fmt::Display for TdfDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::InvalidData(message)
            | Self::UnexpectedEof(message)
            | Self::MissingAbsoluteTimestamp(message) => f.write_str(message),
            Self::UnknownDiffType(diff_type) => {
                write!(f, "Diff array uses an unsupported diff type: {diff_type}")
            }
        }
    }
}

impl Error for TdfDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TdfDecodeError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdfReading {
    pub remote_id: Option<u64>,
    pub id: u16,
    pub name: Option<&'static str>,
    pub timestamp: Option<i64>,
    pub period: Option<f64>,
    pub index: Option<u16>,
    pub raw_data: Vec<u8>,
    pub decoded_payload: Vec<decoders::TdfDecodedPayload>,
}

#[derive(Debug, Default)]
pub struct TdfBlockDecodeResult {
    pub readings: Vec<TdfReading>,
    pub error: Option<TdfDecodeError>,
}

impl TdfBlockDecodeResult {
    pub fn success(&self) -> bool {
        self.error.is_none()
    }
}

fn validate_diff_base_size(tdf_size: u8, base_size: u8) -> TdfDecodeResult<usize> {
    if !tdf_size.is_multiple_of(base_size) {
        return Err(TdfDecodeError::InvalidData("Invalid diff base TDF len"));
    }

    Ok((tdf_size / base_size) as usize)
}

fn diff_data_reconstruct_i16_i8(
    cursor: &mut Cursor<&[u8]>,
    tdf_size: u8,
    diff_num: usize,
    out: &mut Vec<u8>,
) -> TdfDecodeResult<()> {
    let diff_num_fields = validate_diff_base_size(tdf_size, 2)?;
    let mut reconstructed = Vec::with_capacity(diff_num_fields * (diff_num + 1));

    for _ in 0..diff_num_fields {
        reconstructed.push(cursor.read_i16::<LittleEndian>()?);
    }

    for sample_idx in 0..diff_num {
        for field_idx in 0..diff_num_fields {
            let last_val = reconstructed[sample_idx * diff_num_fields + field_idx];
            let diff = cursor.read_i8()? as i16;
            reconstructed.push(last_val.wrapping_add(diff));
        }
    }

    for value in reconstructed {
        out.write_i16::<LittleEndian>(value)?;
    }

    Ok(())
}

fn diff_data_reconstruct_i32_i8(
    cursor: &mut Cursor<&[u8]>,
    tdf_size: u8,
    diff_num: usize,
    out: &mut Vec<u8>,
) -> TdfDecodeResult<()> {
    let diff_num_fields = validate_diff_base_size(tdf_size, 4)?;
    let mut reconstructed = Vec::with_capacity(diff_num_fields * (diff_num + 1));

    for _ in 0..diff_num_fields {
        reconstructed.push(cursor.read_i32::<LittleEndian>()?);
    }

    for sample_idx in 0..diff_num {
        for field_idx in 0..diff_num_fields {
            let last_val = reconstructed[sample_idx * diff_num_fields + field_idx];
            let diff = cursor.read_i8()? as i32;
            reconstructed.push(last_val.wrapping_add(diff));
        }
    }

    for value in reconstructed {
        out.write_i32::<LittleEndian>(value)?;
    }

    Ok(())
}

fn diff_data_reconstruct_i32_i16(
    cursor: &mut Cursor<&[u8]>,
    tdf_size: u8,
    diff_num: usize,
    out: &mut Vec<u8>,
) -> TdfDecodeResult<()> {
    let diff_num_fields = validate_diff_base_size(tdf_size, 4)?;
    let mut reconstructed = Vec::with_capacity(diff_num_fields * (diff_num + 1));

    for _ in 0..diff_num_fields {
        reconstructed.push(cursor.read_i32::<LittleEndian>()?);
    }

    for sample_idx in 0..diff_num {
        for field_idx in 0..diff_num_fields {
            let last_val = reconstructed[sample_idx * diff_num_fields + field_idx];
            let diff = cursor.read_i16::<LittleEndian>()? as i32;
            reconstructed.push(last_val.wrapping_add(diff));
        }
    }

    for value in reconstructed {
        out.write_i32::<LittleEndian>(value)?;
    }

    Ok(())
}

fn decode_payloads(
    tdf_id: u16,
    tdf_size: u8,
    payload_data: &[u8],
) -> TdfDecodeResult<Vec<decoders::TdfDecodedPayload>> {
    if payload_data.is_empty() {
        return Ok(Vec::new());
    }

    let element_size = tdf_size as usize;
    if element_size == 0 || !payload_data.len().is_multiple_of(element_size) {
        return Err(TdfDecodeError::InvalidData(
            "Payload length is not divisible by the element size",
        ));
    }

    let mut decoded = Vec::new();
    for payload in payload_data.chunks_exact(element_size) {
        if let Some(value) = decoders::tdf_decode_payload(tdf_id, tdf_size, payload)? {
            decoded.push(value);
        }
    }

    Ok(decoded)
}

struct TdfReadingParts<'a> {
    remote_id: Option<u64>,
    tdf_id: u16,
    tdf_size: u8,
    buffer_time: Option<i64>,
    array_sample_idx: Option<u16>,
    period: Option<f64>,
    raw_data: Vec<u8>,
    payload_data: &'a [u8],
}

fn tdf_reading(parts: TdfReadingParts<'_>) -> TdfDecodeResult<TdfReading> {
    let TdfReadingParts {
        remote_id,
        tdf_id,
        tdf_size,
        buffer_time,
        array_sample_idx,
        period,
        raw_data,
        payload_data,
    } = parts;

    Ok(TdfReading {
        remote_id,
        id: tdf_id,
        name: decoders::tdf_known_name(tdf_id),
        timestamp: buffer_time,
        period,
        index: array_sample_idx,
        raw_data,
        decoded_payload: decode_payloads(tdf_id, tdf_size, payload_data)?,
    })
}

fn is_padding(block: &[u8], offset: usize) -> bool {
    let Some((&padding_byte, rest)) = block[offset..].split_first() else {
        return false;
    };

    matches!(padding_byte, 0x00 | 0xFF) && rest.iter().all(|value| *value == padding_byte)
}

/// Decode a single TDF block into an in-memory result.
pub fn block_decode(remote_id: Option<u64>, block: &[u8]) -> TdfBlockDecodeResult {
    let mut cursor = Cursor::new(block);
    let mut buffer_time: Option<i64> = None;
    let mut readings = Vec::new();

    macro_rules! try_or_result {
        ($expr:expr) => {
            match $expr {
                Ok(value) => value,
                Err(error) => {
                    return TdfBlockDecodeResult {
                        readings,
                        error: Some(error.into()),
                    }
                }
            }
        };
    }

    while (cursor.position() as usize) < block.len() {
        let offset = cursor.position() as usize;
        if is_padding(block, offset) {
            break;
        }

        if offset + 2 > block.len() {
            return TdfBlockDecodeResult {
                readings,
                error: Some(TdfDecodeError::UnexpectedEof(
                    "Block ended before a complete TDF header could be read",
                )),
            };
        }

        let header = try_or_result!(cursor.read_u16::<LittleEndian>());
        if header == 0xFFFF || header == 0x0000 {
            break;
        }

        if offset + 3 > block.len() {
            return TdfBlockDecodeResult {
                readings,
                error: Some(TdfDecodeError::UnexpectedEof(
                    "Block ended before a complete TDF header could be read",
                )),
            };
        }

        let tdf_id = header & TDF_ID_MASK;
        let time_flags = header & TDF_TIME_MASK;
        let array_flags = header & TDF_ARRAY_MASK;
        let size = try_or_result!(cursor.read_u8());
        let mut array_num = 1;
        let mut array_time_period = 0;
        let mut array_sample_idx = None;
        let mut reconstructed: Option<Vec<u8>> = None;
        let mut payload_start_override = None;
        if size == 0 {
            // Invalid header, remainder of block can't be trusted
            return TdfBlockDecodeResult {
                readings,
                error: Some(TdfDecodeError::InvalidData("TDF of length 0")),
            };
        }
        match time_flags {
            TDF_TIME_NONE => {}
            TDF_TIME_GLOBAL => {
                buffer_time = Some(
                    ((try_or_result!(cursor.read_u32::<LittleEndian>()) as i64) << 16)
                        + (try_or_result!(cursor.read_u16::<LittleEndian>()) as i64),
                );
            }
            TDF_TIME_RELATIVE_U16 => {
                let offset = try_or_result!(cursor.read_u16::<LittleEndian>()) as i64;
                let Some(current_time) = buffer_time else {
                    return TdfBlockDecodeResult {
                        readings,
                        error: Some(TdfDecodeError::MissingAbsoluteTimestamp(
                            "Relative timestamp encountered before an absolute timestamp",
                        )),
                    };
                };
                buffer_time = Some(current_time + offset);
            }
            TDF_TIME_RELATIVE_S24 => {
                let offset = try_or_result!(cursor.read_i24::<LittleEndian>()) as i64;
                let Some(current_time) = buffer_time else {
                    return TdfBlockDecodeResult {
                        readings,
                        error: Some(TdfDecodeError::MissingAbsoluteTimestamp(
                            "Extended relative timestamp encountered before an absolute timestamp",
                        )),
                    };
                };
                buffer_time = Some(current_time + offset);
            }
            _ => {
                panic!("How?");
            }
        }
        match array_flags {
            TDF_ARRAY_NONE => {}
            TDF_ARRAY_TIME => {
                array_num = try_or_result!(cursor.read_u8());
                if array_num == 0 {
                    // Invalid header, remainder of block can't be trusted
                    return TdfBlockDecodeResult {
                        readings,
                        error: Some(TdfDecodeError::InvalidData("Time array of 0 elements")),
                    };
                }
                let period_encoded = try_or_result!(cursor.read_u16::<LittleEndian>());
                let period_masked = period_encoded & TDF_PERIOD_SCALING_VAL_MASK;
                // Handle time period scaling
                array_time_period = period_masked as i64;
                if period_encoded & TDF_PERIOD_SCALING_BIT != 0 {
                    array_time_period *= TDF_PERIOD_SCALING_MULT as i64;
                }
            }
            TDF_ARRAY_DIFF => {
                let diff_info = try_or_result!(cursor.read_u8());
                let period_encoded = try_or_result!(cursor.read_u16::<LittleEndian>());
                let period_masked = period_encoded & TDF_PERIOD_SCALING_VAL_MASK;
                // Handle time period scaling
                array_time_period = period_masked as i64;
                if period_encoded & TDF_PERIOD_SCALING_BIT != 0 {
                    array_time_period *= TDF_PERIOD_SCALING_MULT as i64;
                }
                // Handle diff data
                let diff_type = diff_info >> 6;
                let diff_num = (diff_info & 0x3F) as usize;
                let out_len = size as usize * (1 + diff_num);

                array_num = diff_num as u8 + 1;
                payload_start_override = Some(cursor.position() as usize);
                reconstructed = match diff_type {
                    TDF_DIFF_16_8 => {
                        let mut out: Vec<u8> = Vec::with_capacity(out_len);
                        try_or_result!(diff_data_reconstruct_i16_i8(
                            &mut cursor,
                            size,
                            diff_num,
                            &mut out
                        ));
                        Some(out)
                    }
                    TDF_DIFF_32_8 => {
                        let mut out: Vec<u8> = Vec::with_capacity(out_len);
                        try_or_result!(diff_data_reconstruct_i32_i8(
                            &mut cursor,
                            size,
                            diff_num,
                            &mut out
                        ));
                        Some(out)
                    }
                    TDF_DIFF_32_16 => {
                        let mut out: Vec<u8> = Vec::with_capacity(out_len);
                        try_or_result!(diff_data_reconstruct_i32_i16(
                            &mut cursor,
                            size,
                            diff_num,
                            &mut out
                        ));
                        Some(out)
                    }
                    _ => {
                        return TdfBlockDecodeResult {
                            readings,
                            error: Some(TdfDecodeError::UnknownDiffType(diff_type)),
                        };
                    }
                };
            }
            TDF_ARRAY_IDX => {
                array_num = try_or_result!(cursor.read_u8());
                if array_num == 0 {
                    // Invalid header, remainder of block can't be trusted
                    return TdfBlockDecodeResult {
                        readings,
                        error: Some(TdfDecodeError::InvalidData("Index array of 0 elements")),
                    };
                }
                let base_idx = try_or_result!(cursor.read_u16::<LittleEndian>());
                array_time_period = base_idx as i64;
                array_sample_idx = Some(base_idx);
            }
            _ => {
                panic!("How?");
            }
        }

        let payload_start = payload_start_override.unwrap_or(cursor.position() as usize);
        let period = match array_flags {
            TDF_ARRAY_TIME | TDF_ARRAY_DIFF | TDF_ARRAY_IDX => {
                Some(array_time_period as f64 / 65536.0)
            }
            _ => None,
        };
        let payload_data = match reconstructed {
            Some(ref r) => {
                let reading = try_or_result!(tdf_reading(TdfReadingParts {
                    remote_id,
                    tdf_id,
                    tdf_size: size,
                    buffer_time,
                    array_sample_idx,
                    period,
                    raw_data: r.clone(),
                    payload_data: r,
                }));
                readings.push(reading);
                continue;
            }
            None => {
                let payload_len = size as usize * array_num as usize;
                let payload_end = match payload_start.checked_add(payload_len) {
                    Some(payload_end) => payload_end,
                    None => {
                        return TdfBlockDecodeResult {
                            readings,
                            error: Some(TdfDecodeError::InvalidData("Invalid payload length")),
                        };
                    }
                };
                if payload_end > block.len() {
                    return TdfBlockDecodeResult {
                        readings,
                        error: Some(TdfDecodeError::UnexpectedEof(
                            "Block ended before the payload could be read",
                        )),
                    };
                }
                cursor.set_position(payload_end as u64);
                block[payload_start..payload_end].to_vec()
            }
        };

        let reading = try_or_result!(tdf_reading(TdfReadingParts {
            remote_id,
            tdf_id,
            tdf_size: size,
            buffer_time,
            array_sample_idx,
            period,
            raw_data: payload_data.clone(),
            payload_data: &payload_data,
        }));
        readings.push(reading);
    }

    TdfBlockDecodeResult {
        readings,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_minimum_reading_without_terminator() {
        let block = [0xE7, 0x03, 0x01, 0xAA];

        let result = block_decode(None, &block);

        assert!(result.success());
        assert_eq!(result.readings.len(), 1);
        assert_eq!(result.readings[0].id, 999);
        assert_eq!(result.readings[0].raw_data, [0xAA]);
    }

    #[test]
    fn decodes_marker_like_header_bytes_as_tdf_data() {
        let block = [0x01, 0x02, 0x01, 0xAA];

        let result = block_decode(None, &block);

        assert!(result.success());
        assert_eq!(result.readings.len(), 1);
        assert_eq!(result.readings[0].id, 513);
        assert_eq!(result.readings[0].raw_data, [0xAA]);
    }

    #[test]
    fn accepts_trailing_padding_bytes() {
        for padding_byte in [0x00, 0xFF] {
            let block = [0xE7, 0x03, 0x01, 0xAA, padding_byte, padding_byte];

            let result = block_decode(None, &block);

            assert!(result.success());
            assert_eq!(result.readings.len(), 1);
        }
    }

    #[test]
    fn rejects_mixed_trailing_bytes() {
        let block = [0xE7, 0x03, 0x01, 0xAA, 0xFF, 0x00, 0xFF];

        let result = block_decode(None, &block);

        assert_eq!(result.readings.len(), 1);
        assert!(matches!(
            result.error,
            Some(TdfDecodeError::UnexpectedEof(_))
        ));
    }

    #[test]
    fn rejects_known_payload_with_trailing_bytes() {
        let block = [
            0x36, 0x00, 0x02, // id=54 BATTERY_SOC, element size=2
            87, 0x00, // BATTERY_SOC consumes one byte, leaving one trailing byte
            0x00, 0x00,
        ];

        let result = block_decode(None, &block);

        assert!(matches!(result.error, Some(TdfDecodeError::InvalidData(_))));
        assert!(result.readings.is_empty());
    }

    #[test]
    fn reconstructs_i16_base_i8_diffs_as_little_endian() {
        let input = [
            0x02, 0x01, // 258
            0xFE, 0xFF, // -2
            0x01, // 259
            0xFF, // -3
            0x02, // 261
            0x03, // 0
        ];
        let mut cursor = Cursor::new(input.as_slice());
        let mut out = Vec::new();

        diff_data_reconstruct_i16_i8(&mut cursor, 4, 2, &mut out).unwrap();

        assert_eq!(
            out,
            [
                0x02, 0x01, 0xFE, 0xFF, // base
                0x03, 0x01, 0xFD, 0xFF, // first diff sample
                0x05, 0x01, 0x00, 0x00, // second diff sample
            ]
        );
    }

    #[test]
    fn reconstructs_i32_base_i8_diffs_as_little_endian() {
        let input = [
            0x04, 0x03, 0x02, 0x01, // 0x01020304
            0xFC, // -4
            0x05, // +5
        ];
        let mut cursor = Cursor::new(input.as_slice());
        let mut out = Vec::new();

        diff_data_reconstruct_i32_i8(&mut cursor, 4, 2, &mut out).unwrap();

        assert_eq!(
            out,
            [
                0x04, 0x03, 0x02, 0x01, // base
                0x00, 0x03, 0x02, 0x01, // first diff sample
                0x05, 0x03, 0x02, 0x01, // second diff sample
            ]
        );
    }

    #[test]
    fn reconstructs_i32_base_i16_diffs_as_little_endian() {
        let input = [
            0xE8, 0x03, 0x00, 0x00, // 1000
            0x18, 0xFC, 0xFF, 0xFF, // -1000
            0x2C, 0x01, // +300
            0xD4, 0xFE, // -300
            0xCE, 0xFF, // -50
            0x32, 0x00, // +50
        ];
        let mut cursor = Cursor::new(input.as_slice());
        let mut out = Vec::new();

        diff_data_reconstruct_i32_i16(&mut cursor, 8, 2, &mut out).unwrap();

        assert_eq!(
            out,
            [
                0xE8, 0x03, 0x00, 0x00, // 1000
                0x18, 0xFC, 0xFF, 0xFF, // -1000
                0x14, 0x05, 0x00, 0x00, // 1300
                0xEC, 0xFA, 0xFF, 0xFF, // -1300
                0xE2, 0x04, 0x00, 0x00, // 1250
                0x1E, 0xFB, 0xFF, 0xFF, // -1250
            ]
        );
    }

    #[test]
    fn rejects_zero_element_index_arrays() {
        let block = [
            0xE7, 0x33, 0x01, // id=999, idx array, element size=1
            0x00, 0x34, 0x12, // array count=0, base index=0x1234
            0x00, 0x00,
        ];

        let result = block_decode(None, &block);

        assert!(matches!(result.error, Some(TdfDecodeError::InvalidData(_))));
        assert!(result.readings.is_empty());
    }

    #[test]
    fn rejects_zero_element_time_arrays() {
        let block = [
            0xE7, 0x13, 0x01, // id=999, time array, element size=1
            0x00, 0x00, 0x40, // array count=0, period=0.25s
            0x00, 0x00,
        ];

        let result = block_decode(None, &block);

        assert!(matches!(result.error, Some(TdfDecodeError::InvalidData(_))));
        assert!(result.readings.is_empty());
    }
}
