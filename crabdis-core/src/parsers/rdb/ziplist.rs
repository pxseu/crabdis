use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Parsed ziplist entry value.
#[derive(Debug, Clone)]
enum ZiplistEntry {
    String(Arc<str>),
    Integer(i64),
}

impl ZiplistEntry {
    /// Converts the entry to a string representation.
    fn to_arc_str(&self) -> Arc<str> {
        match self {
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string().into(),
        }
    }
}

/// Parses a ziplist and returns all entries.
///
/// Ziplist format:
/// - 4 bytes: total bytes (little-endian)
/// - 2 bytes: number of entries (little-endian)
/// - entries...
/// - 1 byte: 0xFF end marker
fn parse(data: &[u8]) -> Result<Vec<ZiplistEntry>> {
    if data.len() < 11 {
        // Minimum: 4 (zlbytes) + 4 (zltail) + 2 (zllen) + 1 (end)
        return Ok(Vec::new());
    }

    // Skip zlbytes (4 bytes) and zltail (4 bytes)
    // Read zllen (2 bytes)
    let num_entries = u16::from_le_bytes([data[8], data[9]]) as usize;

    let mut entries = Vec::with_capacity(num_entries);
    let mut pos = 10; // Start after header

    while pos < data.len() && data[pos] != super::symbols::ZIPLIST_END {
        let (entry, consumed) = parse_entry(data, pos)?;
        entries.push(entry);
        pos += consumed;
    }

    Ok(entries)
}

/// Parses a ziplist as a hash (alternating key-value pairs).
///
/// Returns a `HashMap` where keys and values are converted to `Value::String`.
///
/// # Errors
///
/// Returns an error if the ziplist data is malformed or has odd entries.
pub fn parse_hash(data: &[u8]) -> Result<HashMap<Value, Value>> {
    let entries = parse(data)?;
    let mut map = HashMap::with_capacity(entries.len() / 2);

    let mut iter = entries.into_iter();
    while let Some(key) = iter.next() {
        let value = iter.next().ok_or_else(|| {
            IoError::new(
                ErrorKind::InvalidData,
                "Ziplist hash has odd number of entries",
            )
        })?;

        map.insert(
            Value::String(key.to_arc_str()),
            Value::String(value.to_arc_str()),
        );
    }

    Ok(map)
}

/// Parses a single ziplist entry and returns `(entry, bytes_consumed)`.
fn parse_entry(data: &[u8], pos: usize) -> Result<(ZiplistEntry, usize)> {
    if pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Ziplist entry out of bounds").into());
    }

    // Parse prevlen (1 or 5 bytes)
    let (prevlen_size, _prevlen) = parse_prevlen(data, pos)?;
    let encoding_pos = pos + prevlen_size;

    if encoding_pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Ziplist encoding out of bounds").into());
    }

    let encoding = data[encoding_pos];

    // Parse based on encoding
    let (entry, data_size) = parse_encoding(data, encoding_pos, encoding)?;

    Ok((entry, prevlen_size + data_size))
}

/// Parses the prevlen field (previous entry length).
fn parse_prevlen(data: &[u8], pos: usize) -> Result<(usize, u32)> {
    if pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Prevlen out of bounds").into());
    }

    let first = data[pos];
    if first < super::symbols::ZIPLIST_BIGLEN {
        Ok((1, u32::from(first)))
    } else {
        // 5-byte encoding
        if pos + 5 > data.len() {
            return Err(
                IoError::new(ErrorKind::InvalidData, "Prevlen 5-byte out of bounds").into(),
            );
        }
        let prevlen =
            u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]);
        Ok((5, prevlen))
    }
}

/// Parses the encoding and data portion of a ziplist entry.
fn parse_encoding(data: &[u8], pos: usize, encoding: u8) -> Result<(ZiplistEntry, usize)> {
    let top_two = encoding & 0xC0;

    match top_two {
        // String encodings
        super::symbols::ZIPLIST_STR_6BIT => {
            // 00xxxxxx - 6-bit length string
            let len = (encoding & 0x3F) as usize;
            if pos + 1 + len > data.len() {
                return Err(
                    IoError::new(ErrorKind::InvalidData, "Ziplist string too short").into(),
                );
            }
            let s = String::from_utf8_lossy(&data[pos + 1..pos + 1 + len]);
            Ok((ZiplistEntry::String(s.into_owned().into()), 1 + len))
        }
        super::symbols::ZIPLIST_STR_14BIT => {
            // 01xxxxxx - 14-bit length string
            if pos + 2 > data.len() {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "Ziplist 14-bit length too short",
                )
                .into());
            }
            let len = (((encoding & 0x3F) as usize) << 8) | (data[pos + 1] as usize);
            if pos + 2 + len > data.len() {
                return Err(
                    IoError::new(ErrorKind::InvalidData, "Ziplist string too short").into(),
                );
            }
            let s = String::from_utf8_lossy(&data[pos + 2..pos + 2 + len]);
            Ok((ZiplistEntry::String(s.into_owned().into()), 2 + len))
        }
        super::symbols::ZIPLIST_STR_32BIT => {
            // 10000000 - 32-bit length string
            if pos + 5 > data.len() {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "Ziplist 32-bit length too short",
                )
                .into());
            }
            let len =
                u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]])
                    as usize;
            if pos + 5 + len > data.len() {
                return Err(
                    IoError::new(ErrorKind::InvalidData, "Ziplist string too short").into(),
                );
            }
            let s = String::from_utf8_lossy(&data[pos + 5..pos + 5 + len]);
            Ok((ZiplistEntry::String(s.into_owned().into()), 5 + len))
        }
        _ => {
            // Integer encodings (top 2 bits are 11)
            parse_integer_encoding(data, pos, encoding)
        }
    }
}

/// Parses integer encodings in ziplist.
#[allow(clippy::cast_possible_wrap)]
fn parse_integer_encoding(data: &[u8], pos: usize, encoding: u8) -> Result<(ZiplistEntry, usize)> {
    match encoding {
        super::symbols::ZIPLIST_INT_16 => {
            // 11000000 - int16
            if pos + 3 > data.len() {
                return Err(IoError::new(ErrorKind::InvalidData, "Ziplist int16 too short").into());
            }
            let val = i16::from_le_bytes([data[pos + 1], data[pos + 2]]);
            Ok((ZiplistEntry::Integer(i64::from(val)), 3))
        }
        super::symbols::ZIPLIST_INT_32 => {
            // 11010000 - int32
            if pos + 5 > data.len() {
                return Err(IoError::new(ErrorKind::InvalidData, "Ziplist int32 too short").into());
            }
            let val =
                i32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]]);
            Ok((ZiplistEntry::Integer(i64::from(val)), 5))
        }
        super::symbols::ZIPLIST_INT_64 => {
            // 11100000 - int64
            if pos + 9 > data.len() {
                return Err(IoError::new(ErrorKind::InvalidData, "Ziplist int64 too short").into());
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
            Ok((ZiplistEntry::Integer(val), 9))
        }
        super::symbols::ZIPLIST_INT_24 => {
            // 11110000 - 24-bit signed int
            if pos + 4 > data.len() {
                return Err(IoError::new(ErrorKind::InvalidData, "Ziplist int24 too short").into());
            }
            // Sign extend from 24 to 32 bits
            let val = i32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], 0]);
            // If the sign bit (bit 23) is set, extend the sign
            let val = if val & 0x0080_0000 != 0 {
                val | (0xFF00_0000_u32 as i32)
            } else {
                val
            };
            Ok((ZiplistEntry::Integer(i64::from(val)), 4))
        }
        super::symbols::ZIPLIST_INT_8 => {
            // 11111110 - 8-bit signed int
            if pos + 2 > data.len() {
                return Err(IoError::new(ErrorKind::InvalidData, "Ziplist int8 too short").into());
            }
            let val = data[pos + 1] as i8;
            Ok((ZiplistEntry::Integer(i64::from(val)), 2))
        }
        _ if (0xF1..=0xFD).contains(&encoding) => {
            // 1111xxxx - 4-bit unsigned int (0-12, stored as 1-13)
            let val = i64::from((encoding & 0x0F) - 1);
            Ok((ZiplistEntry::Integer(val), 1))
        }
        _ => Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Unknown ziplist encoding: {encoding:#04x}"),
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::rdb;

    fn make_ziplist(entries: &[u8]) -> Vec<u8> {
        // Create a minimal ziplist
        let total_len = 11 + entries.len(); // header (10) + entries + end (1)
        let mut data = Vec::with_capacity(total_len);

        // zlbytes (4 bytes, little-endian)
        #[allow(clippy::cast_possible_truncation)]
        data.extend_from_slice(&(total_len as u32).to_le_bytes());
        // zltail (4 bytes) - we don't need accurate value for parsing
        data.extend_from_slice(&[0u8; 4]);
        // zllen (2 bytes) - we let the parser count
        data.extend_from_slice(&[0u8; 2]);
        // entries
        data.extend_from_slice(entries);
        // end marker
        data.push(rdb::symbols::ZIPLIST_END);

        data
    }

    #[test]
    fn test_parse_empty() {
        let data = make_ziplist(&[]);
        let entries = parse(&data).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn test_parse_6bit_string() {
        // prevlen=0, encoding=0x05 (6-bit string, len=5), "hello"
        let entries_data = [0x00, 0x05, b'h', b'e', b'l', b'l', b'o'];
        let data = make_ziplist(&entries_data);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ZiplistEntry::String(s) if s.as_ref() == "hello"));
    }

    #[test]
    fn test_parse_small_int() {
        // prevlen=0, encoding=0xF2 (4-bit int, value 1)
        let entries_data = [0x00, 0xF2];
        let data = make_ziplist(&entries_data);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ZiplistEntry::Integer(1)));
    }

    #[test]
    fn test_parse_int8() {
        // prevlen=0, encoding=0xFE (int8), value=42
        let entries_data = [0x00, rdb::symbols::ZIPLIST_INT_8, 42];
        let data = make_ziplist(&entries_data);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ZiplistEntry::Integer(42)));
    }

    #[test]
    fn test_parse_int16() {
        // prevlen=0, encoding=0xC0 (int16), value=1000
        let entries_data = [0x00, rdb::symbols::ZIPLIST_INT_16, 0xE8, 0x03];
        let data = make_ziplist(&entries_data);
        let entries = parse(&data).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(&entries[0], ZiplistEntry::Integer(1000)));
    }

    #[test]
    fn test_parse_hash() {
        // Two entries: "key" and "value"
        let entries_data = [
            0x00, 0x03, b'k', b'e', b'y', // prevlen=0, 6-bit string "key"
            0x05, 0x05, b'v', b'a', b'l', b'u', b'e', // prevlen=5, 6-bit string "value"
        ];
        let data = make_ziplist(&entries_data);
        let map = parse_hash(&data).unwrap();

        assert_eq!(map.len(), 1);
        let key = Value::String("key".into());
        assert!(map.contains_key(&key));
        assert_eq!(map.get(&key), Some(&Value::String("value".into())));
    }
}
