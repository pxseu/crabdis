use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Parses a zipmap-encoded hash.
///
/// Zipmap format (very old, Redis < 2.6):
/// - 1 byte: zmlen (number of entries, or 254 if >= 254)
/// - entries: [key-len][key][val-len][free][val]...
/// - 1 byte: 0xFF end marker
///
/// Length encoding:
/// - 0-253: literal length
/// - 254: next 4 bytes are the length (little-endian)
///
/// # Errors
///
/// Returns an error if the zipmap data is malformed.
pub fn parse(data: &[u8]) -> Result<HashMap<Value, Value>> {
    if data.is_empty() {
        return Ok(HashMap::new());
    }

    let mut map = HashMap::new();
    let mut pos = 1; // Skip zmlen byte

    while pos < data.len() && data[pos] != super::symbols::ZIPMAP_END {
        // Parse key
        let (key, key_consumed) = parse_string(data, pos)?;
        pos += key_consumed;

        if pos >= data.len() || data[pos] == super::symbols::ZIPMAP_END {
            return Err(
                IoError::new(ErrorKind::InvalidData, "Zipmap missing value after key").into(),
            );
        }

        // Parse value (includes 'free' byte after length)
        let (value, value_consumed) = parse_value(data, pos)?;
        pos += value_consumed;

        map.insert(Value::String(key), Value::String(value));
    }

    Ok(map)
}

/// Parses a zipmap string (key or value data).
fn parse_string(data: &[u8], pos: usize) -> Result<(Arc<str>, usize)> {
    if pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Zipmap string out of bounds").into());
    }

    let (len, len_size) = parse_length(data, pos)?;
    let data_start = pos + len_size;

    if data_start + len > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Zipmap string too short").into());
    }

    let s = String::from_utf8_lossy(&data[data_start..data_start + len]);
    Ok((s.into_owned().into(), len_size + len))
}

/// Parses a zipmap value (length + free byte + data).
fn parse_value(data: &[u8], pos: usize) -> Result<(Arc<str>, usize)> {
    if pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Zipmap value out of bounds").into());
    }

    let (len, len_size) = parse_length(data, pos)?;
    let free_pos = pos + len_size;

    if free_pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Zipmap missing free byte").into());
    }

    let free = data[free_pos] as usize;
    let data_start = free_pos + 1;

    if data_start + len + free > data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Zipmap value too short").into());
    }

    let s = String::from_utf8_lossy(&data[data_start..data_start + len]);
    // Total consumed: length bytes + free byte + actual data + free space
    Ok((s.into_owned().into(), len_size + 1 + len + free))
}

/// Parses a zipmap length field.
fn parse_length(data: &[u8], pos: usize) -> Result<(usize, usize)> {
    if pos >= data.len() {
        return Err(IoError::new(ErrorKind::InvalidData, "Zipmap length out of bounds").into());
    }

    let first = data[pos];
    if first < super::symbols::ZIPMAP_BIGLEN {
        Ok((first as usize, 1))
    } else {
        // 5-byte encoding (0xFE + 4 bytes)
        if pos + 5 > data.len() {
            return Err(
                IoError::new(ErrorKind::InvalidData, "Zipmap 5-byte length too short").into(),
            );
        }
        let len = u32::from_le_bytes([data[pos + 1], data[pos + 2], data[pos + 3], data[pos + 4]])
            as usize;
        Ok((len, 5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::rdb;

    fn make_zipmap(entries: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0); // zmlen (we don't validate this)
        data.extend_from_slice(entries);
        data.push(rdb::symbols::ZIPMAP_END);
        data
    }

    #[test]
    fn test_parse_empty() {
        let data = make_zipmap(&[]);
        let map = parse(&data).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_single_entry() {
        // key: "key" (len=3), value: "val" (len=3, free=0)
        let entries = [
            3, b'k', b'e', b'y', // key length + key
            3, 0, b'v', b'a', b'l', // value length + free + value
        ];
        let data = make_zipmap(&entries);
        let map = parse(&data).unwrap();

        assert_eq!(map.len(), 1);
        let key = Value::String("key".into());
        assert!(map.contains_key(&key));
        assert_eq!(map.get(&key), Some(&Value::String("val".into())));
    }

    #[test]
    fn test_parse_multiple_entries() {
        // key1: "a", value1: "1"
        // key2: "bb", value2: "22"
        let entries = [
            1, b'a', // key1
            1, 0, b'1', // value1
            2, b'b', b'b', // key2
            2, 0, b'2', b'2', // value2
        ];
        let data = make_zipmap(&entries);
        let map = parse(&data).unwrap();

        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get(&Value::String("a".into())),
            Some(&Value::String("1".into()))
        );
        assert_eq!(
            map.get(&Value::String("bb".into())),
            Some(&Value::String("22".into()))
        );
    }

    #[test]
    fn test_parse_with_free_space() {
        // key: "k", value: "v" with 2 bytes free space
        let entries = [
            1, b'k', // key
            1, 2, b'v', b'X', b'X', // value with free=2 (X bytes are ignored)
        ];
        let data = make_zipmap(&entries);
        let map = parse(&data).unwrap();

        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get(&Value::String("k".into())),
            Some(&Value::String("v".into()))
        );
    }
}
