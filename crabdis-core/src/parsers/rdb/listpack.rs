use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Parsed listpack entry value.
#[derive(Debug, Clone)]
enum ListpackEntry {
    String(Arc<str>),
    Integer(i64),
}

impl ListpackEntry {
    /// Converts the entry to a string representation.
    fn to_arc_str(&self) -> Arc<str> {
        match self {
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string().into(),
        }
    }
}

/// Parses a listpack and returns all entries.
///
/// Listpack format:
/// - 4 bytes: total bytes (little-endian)
/// - 2 bytes: number of elements (little-endian)
/// - entries...
/// - 1 byte: 0xFF end marker
fn parse(data: &[u8]) -> Result<Vec<ListpackEntry>> {
    if data.len() < 7 {
        // Minimum: 4 (total) + 2 (num) + 1 (end)
        return Ok(Vec::new());
    }

    let num_elements = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut entries = Vec::with_capacity(num_elements);
    let mut pos = 6; // Start after header

    while pos < data.len() && data[pos] != super::symbols::LISTPACK_END {
        let (entry, consumed) = parse_entry(data, pos)?;
        entries.push(entry);
        pos += consumed;
    }

    Ok(entries)
}

/// Parses a listpack as a hash (alternating key-value pairs).
///
/// # Errors
///
/// Returns an error if the listpack data is malformed or has odd entries.
pub fn parse_hash(data: &[u8]) -> Result<HashMap<Value, Value>> {
    let entries = parse(data)?;
    let mut map = HashMap::with_capacity(entries.len() / 2);

    let mut iter = entries.into_iter();
    while let Some(key) = iter.next() {
        let value = iter.next().ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidData,
                "Listpack hash has odd number of entries",
            )
        })?;

        map.insert(
            Value::String(key.to_arc_str()),
            Value::String(value.to_arc_str()),
        );
    }

    Ok(map)
}

/// Parses a single listpack entry and returns `(entry, bytes_consumed)`.
fn parse_entry(data: &[u8], pos: usize) -> Result<(ListpackEntry, usize)> {
    if pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack entry out of bounds").into());
    }

    let first_byte = data[pos];

    // Small positive integer (0-127)
    if first_byte <= 0x7F {
        let backlen = backlen_size(1);
        return Ok((ListpackEntry::Integer(i64::from(first_byte)), 1 + backlen));
    }

    // 6-bit string (0x80 - 0xBF)
    if first_byte & 0xC0 == 0x80 {
        return parse_6bit_string(data, pos, first_byte);
    }

    // 13-bit signed integer (0xC0 - 0xDF)
    if first_byte & 0xE0 == 0xC0 {
        return parse_13bit_int(data, pos, first_byte);
    }

    // Check for specific encodings
    parse_special_encoding(data, pos, first_byte)
}

/// Parses a 6-bit length string entry.
fn parse_6bit_string(data: &[u8], pos: usize, first_byte: u8) -> Result<(ListpackEntry, usize)> {
    let str_len = (first_byte & 0x3F) as usize;
    if pos + 1 + str_len > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack string too short").into());
    }
    let s = String::from_utf8_lossy(&data[pos + 1..pos + 1 + str_len]);
    let entry_len = 1 + str_len;
    let backlen = backlen_size(entry_len);
    Ok((
        ListpackEntry::String(s.into_owned().into()),
        entry_len + backlen,
    ))
}

/// Parses a 13-bit signed integer entry.
#[allow(clippy::cast_possible_wrap)]
fn parse_13bit_int(data: &[u8], pos: usize, first_byte: u8) -> Result<(ListpackEntry, usize)> {
    if pos + 2 > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack int13 too short").into());
    }
    // Combine into 13-bit value
    let val = (i16::from(first_byte & 0x1F) << 8) | i16::from(data[pos + 1]);
    // Sign extend from 13 bits
    let val = if val & 0x1000 != 0 {
        val | (0xE000_u16 as i16)
    } else {
        val
    };
    let backlen = backlen_size(2);
    Ok((ListpackEntry::Integer(i64::from(val)), 2 + backlen))
}

/// Parses special encoding entries (12-bit string, 32-bit string, various
/// integers).
fn parse_special_encoding(
    data: &[u8],
    pos: usize,
    first_byte: u8,
) -> Result<(ListpackEntry, usize)> {
    // 12-bit string (0xE0-0xEF): 1110xxxx yyyyyyyy
    if first_byte & 0xF0 == 0xE0 {
        return parse_12bit_string(data, pos, first_byte);
    }

    match first_byte {
        // 32-bit string (0xF0)
        0xF0 => parse_32bit_string(data, pos),

        // 16-bit signed integer (0xF1)
        0xF1 => parse_int16(data, pos),

        // 24-bit signed integer (0xF2)
        0xF2 => parse_int24(data, pos),

        // 32-bit signed integer (0xF3)
        0xF3 => parse_int32(data, pos),

        // 64-bit signed integer (0xF4)
        0xF4 => parse_int64(data, pos),

        _ => Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Unknown listpack encoding: {first_byte:#04x}"),
        )
        .into()),
    }
}

/// Parses a 12-bit length string entry.
///
/// Format: 1110xxxx yyyyyyyy <string>
/// where 12-bit length = (xxxx << 8) | yyyyyyyy
fn parse_12bit_string(data: &[u8], pos: usize, first_byte: u8) -> Result<(ListpackEntry, usize)> {
    if pos + 1 >= data.len() {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "Listpack 12-bit string header short",
        )
        .into());
    }
    // Top 4 bits from first byte, bottom 8 bits from second byte
    let str_len = ((usize::from(first_byte & 0x0F)) << 8) | (data[pos + 1] as usize);
    if pos + 2 + str_len > data.len() {
        return Err(
            IoError::new(ErrorKind::InvalidData, "Listpack 12-bit string too short").into(),
        );
    }
    let s = String::from_utf8_lossy(&data[pos + 2..pos + 2 + str_len]);
    let entry_len = 2 + str_len;
    let backlen = backlen_size(entry_len);
    Ok((
        ListpackEntry::String(s.into_owned().into()),
        entry_len + backlen,
    ))
}

/// Parses a 32-bit length string entry.
fn parse_32bit_string(data: &[u8], pos: usize) -> Result<(ListpackEntry, usize)> {
    if pos + 5 > data.len() {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "Listpack 32-bit string header short",
        )
        .into());
    }
    let str_len =
        u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]) as usize;
    if pos + 5 + str_len > data.len() {
        return Err(
            IoError::new(ErrorKind::InvalidData, "Listpack 32-bit string too short").into(),
        );
    }
    let s = String::from_utf8_lossy(&data[pos + 5..pos + 5 + str_len]);
    let entry_len = 5 + str_len;
    let backlen = backlen_size(entry_len);
    Ok((
        ListpackEntry::String(s.into_owned().into()),
        entry_len + backlen,
    ))
}

/// Parses a 16-bit signed integer entry.
fn parse_int16(data: &[u8], pos: usize) -> Result<(ListpackEntry, usize)> {
    if pos + 3 > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack int16 too short").into());
    }
    let val = i16::from_le_bytes([data[pos + 1], data[pos + 2]]);
    let backlen = backlen_size(3);
    Ok((ListpackEntry::Integer(i64::from(val)), 3 + backlen))
}

/// Parses a 24-bit signed integer entry.
#[allow(clippy::cast_possible_wrap)]
fn parse_int24(data: &[u8], pos: usize) -> Result<(ListpackEntry, usize)> {
    if pos + 4 > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack int24 too short").into());
    }
    // Sign extend from 24 bits
    let val = i32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], 0]);
    let val = if val & 0x0080_0000 != 0 {
        val | (0xFF00_0000_u32 as i32)
    } else {
        val
    };
    let backlen = backlen_size(4);
    Ok((ListpackEntry::Integer(i64::from(val)), 4 + backlen))
}

/// Parses a 32-bit signed integer entry.
fn parse_int32(data: &[u8], pos: usize) -> Result<(ListpackEntry, usize)> {
    if pos + 5 > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack int32 too short").into());
    }
    let val = i32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]);
    let backlen = backlen_size(5);
    Ok((ListpackEntry::Integer(i64::from(val)), 5 + backlen))
}

/// Parses a 64-bit signed integer entry.
fn parse_int64(data: &[u8], pos: usize) -> Result<(ListpackEntry, usize)> {
    if pos + 9 > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Listpack int64 too short").into());
    }
    let val = i64::from_le_bytes([
        data[pos + 1],
        data[pos + 2],
        data[pos + 3],
        data[pos + 4],
        data[pos + 5],
        data[pos + 6],
        data[pos + 7],
        data[pos + 8],
    ]);
    let backlen = backlen_size(9);
    Ok((ListpackEntry::Integer(val), 9 + backlen))
}

/// Calculate listpack backlen size based on entry length.
const fn backlen_size(entry_len: usize) -> usize {
    if entry_len <= 127 {
        1
    } else if entry_len <= 16383 {
        2
    } else if entry_len <= 2_097_151 {
        3
    } else if entry_len <= 268_435_455 {
        4
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::rdb;

    fn make_listpack(entries: &[u8]) -> Vec<u8> {
        let total_len = 7 + entries.len();
        let mut data = Vec::with_capacity(total_len);

        // total-bytes (4 bytes, little-endian)
        #[allow(clippy::cast_possible_truncation)]
        data.extend_from_slice(&(total_len as u32).to_le_bytes());
        // num-elements (2 bytes) - we don't validate this
        data.extend_from_slice(&[0u8; 2]);
        // entries
        data.extend_from_slice(entries);
        // end marker
        data.push(rdb::symbols::LISTPACK_END);

        data
    }

    #[test]
    fn test_parse_empty() {
        let data = make_listpack(&[]);
        let entries = parse(&data).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_small_int() {
        // Small int 42, backlen 1
        let data = make_listpack(&[42, 1]);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ListpackEntry::Integer(42)));
    }

    #[test]
    fn test_parse_6bit_string() {
        // 6-bit string "hi" (0x80 | 2 = 0x82), backlen 1
        let data = make_listpack(&[0x82, b'h', b'i', 3]);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ListpackEntry::String(s) if s.as_ref() == "hi"));
    }

    #[test]
    fn test_parse_hash() {
        // Two entries forming a hash: "name" -> "pxseu"
        // Entry 1: 6-bit string "name" (0x84), backlen
        // Entry 2: 6-bit string "pxseu" (0x85), backlen
        let data = make_listpack(&[
            0x84, b'n', b'a', b'm', b'e', 5, // "name" + backlen
            0x85, b'p', b'x', b's', b'e', b'u', 6, // "pxseu" + backlen
        ]);
        let map = parse_hash(&data).unwrap();

        assert_eq!(map.len(), 1);
        let key = Value::String("name".into());
        assert!(map.contains_key(&key));
        assert_eq!(map.get(&key), Some(&Value::String("pxseu".into())));
    }

    #[test]
    fn test_parse_int16() {
        // 16-bit int (0xF1), value 1000 (0x03E8), backlen
        let data = make_listpack(&[0xF1, 0xE8, 0x03, 3]);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ListpackEntry::Integer(1000)));
    }

    #[test]
    fn test_parse_negative_int13() {
        // 13-bit signed int -1 (0xC0 | 0x1F = 0xDF, 0xFF)
        // Actually, -1 in 13-bit is 0x1FFF, so encoding is 0xDF 0xFF
        let data = make_listpack(&[0xDF, 0xFF, 2]);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ListpackEntry::Integer(-1)));
    }

    #[test]
    fn test_parse_12bit_string() {
        // 12-bit string "hello world!" (12 chars)
        // Encoding: 0xE0 | (len >> 8), len & 0xFF = 0xE0, 0x0C
        // str_len = 12 (0x00C)
        let mut entry = vec![0xE0, 0x0C]; // 12-bit length header
        entry.extend_from_slice(b"hello world!");
        entry.push(14); // backlen (2 + 12 = 14, fits in 1 byte)
        let data = make_listpack(&entry);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ListpackEntry::String(s) if s.as_ref() == "hello world!"));
    }

    #[test]
    fn test_parse_12bit_string_longer() {
        // 12-bit string with length > 255 (testing the 12-bit range)
        // len = 300 (0x12C) => first_byte = 0xE1, second_byte = 0x2C
        let test_str = "a".repeat(300);
        let mut entry = vec![0xE1, 0x2C]; // 12-bit length = (1 << 8) | 0x2C = 300
        entry.extend_from_slice(test_str.as_bytes());
        entry.push(0x82); // backlen for 302 bytes (2 + 300), encoded as 2 bytes
        entry.push(0x02);
        let data = make_listpack(&entry);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ListpackEntry::String(s) if s.as_ref() == test_str));
    }
}
