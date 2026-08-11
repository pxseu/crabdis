//! LZF compression and decompression implementation for RDB files.
//!
//! LZF is a simple, fast compression algorithm used by Redis to compress
//! strings in RDB files. It uses a combination of literal runs and
//! back-references.
//!
//! LZF, originally by Apple Inc., released under the BSD license.
//! <https://github.com/lzfse/lzfse>

use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Hash table size for compression (must be power of 2).
const HASH_SIZE: usize = 1 << 14; // 16384

/// Maximum back-reference offset (0x1FFF = 8191, 13 bits).
const MAX_OFFSET: usize = 0x1FFF;

/// Maximum back-reference length.
const MAX_REF_LEN: usize = 264; // 7 + 255 + 2

/// Maximum literal run length.
const MAX_LITERAL: usize = 32;

/// Minimum string length to attempt compression.
pub const MIN_COMPRESS_LEN: usize = 20;

/// Computes hash for 3 bytes at position.
#[inline]
fn hash(data: &[u8], pos: usize) -> usize {
    let v =
        u32::from(data[pos]) | (u32::from(data[pos + 1]) << 8) | (u32::from(data[pos + 2]) << 16);
    ((v.wrapping_mul(0x1E35_A7BD)) >> 18) as usize & (HASH_SIZE - 1)
}

/// Compresses data using LZF algorithm.
///
/// Returns `None` if compression doesn't reduce size (incompressible data).
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn compress(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() < 4 {
        return None;
    }

    let mut output = Vec::with_capacity(input.len());
    let mut hash_table = vec![usize::MAX; HASH_SIZE];

    let mut ip = 0;
    let mut anchor = 0;

    while ip < input.len() - 2 {
        let h = hash(input, ip);
        let ref_pos = hash_table[h];
        hash_table[h] = ip;

        if ref_pos != usize::MAX
            && ip > ref_pos
            && ip - ref_pos <= MAX_OFFSET
            && input[ref_pos] == input[ip]
            && input[ref_pos + 1] == input[ip + 1]
            && input[ref_pos + 2] == input[ip + 2]
        {
            if ip > anchor {
                emit_literals(&mut output, &input[anchor..ip]);
            }

            let offset = ip - ref_pos;
            let mut match_len = 3;
            let max_len = (input.len() - ip).min(MAX_REF_LEN);
            while match_len < max_len && input[ref_pos + match_len] == input[ip + match_len] {
                match_len += 1;
            }

            emit_backref(&mut output, offset, match_len);

            let match_end = ip + match_len;
            ip += 1;
            while ip < match_end.saturating_sub(2) && ip < input.len() - 2 {
                let h = hash(input, ip);
                hash_table[h] = ip;
                ip += 1;
            }
            ip = match_end;
            anchor = ip;
        } else {
            ip += 1;
        }
    }

    if anchor < input.len() {
        emit_literals(&mut output, &input[anchor..]);
    }

    if output.len() < input.len() {
        Some(output)
    } else {
        None
    }
}

/// Emits literals, splitting into chunks if longer than `MAX_LITERAL`.
#[allow(clippy::cast_possible_truncation)]
fn emit_literals(output: &mut Vec<u8>, data: &[u8]) {
    let mut pos = 0;
    while pos < data.len() {
        let chunk_len = (data.len() - pos).min(MAX_LITERAL);
        output.push((chunk_len - 1) as u8);
        output.extend_from_slice(&data[pos..pos + chunk_len]);
        pos += chunk_len;
    }
}

/// Emits a back-reference to the output.
#[allow(clippy::cast_possible_truncation)]
fn emit_backref(output: &mut Vec<u8>, offset: usize, len: usize) {
    debug_assert!(len >= 3 && (1..=MAX_OFFSET).contains(&offset));

    let len_code = len - 2;
    let offset_minus_1 = offset - 1;

    if len_code < 7 {
        output.push(((len_code << 5) | (offset_minus_1 >> 8)) as u8);
    } else {
        output.push((7 << 5) as u8 | (offset_minus_1 >> 8) as u8);
        output.push((len_code - 7) as u8);
    }
    output.push((offset_minus_1 & 0xFF) as u8);
}

/// Decompresses LZF-compressed data.
///
/// # Arguments
///
/// * `compressed` - The compressed data bytes
/// * `uncompressed_len` - Expected length of decompressed data
///
/// # Errors
///
/// Returns an error if:
/// - Compressed data is malformed
/// - Back-reference points outside the output buffer
/// - Decompressed size doesn't match expected length
#[allow(clippy::cast_possible_truncation)]
pub fn decompress(compressed: &[u8], uncompressed_len: usize) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(uncompressed_len);
    let mut i = 0;

    while i < compressed.len() {
        let ctrl = compressed[i];
        i += 1;

        if ctrl < 32 {
            // Literal run: copy (ctrl + 1) bytes directly
            let len = (ctrl as usize) + 1;

            if i + len > compressed.len() {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "LZF: unexpected end of compressed data in literal run",
                )
                .into());
            }

            output.extend_from_slice(&compressed[i..i + len]);
            i += len;
        } else {
            // Back-reference
            let mut len = (ctrl >> 5) as usize;
            let mut offset = ((ctrl & 0x1F) as usize) << 8;

            if len == 7 {
                // Length is 7+, read additional byte for extra length
                if i >= compressed.len() {
                    return Err(IoError::new(
                        ErrorKind::InvalidData,
                        "LZF: unexpected end of compressed data reading extended length",
                    )
                    .into());
                }
                len += compressed[i] as usize;
                i += 1;
            }

            // Read offset low byte
            if i >= compressed.len() {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "LZF: unexpected end of compressed data reading offset",
                )
                .into());
            }
            offset |= compressed[i] as usize;
            i += 1;

            // Offset is 1-based (offset 0 means 1 byte back)
            offset += 1;

            // Length is encoded as (length - 2), so actual length is len + 2
            len += 2;

            if offset > output.len() {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    format!(
                        "LZF: back-reference offset {} exceeds output length {}",
                        offset,
                        output.len()
                    ),
                )
                .into());
            }

            // Copy bytes from back-reference
            // We need to copy byte-by-byte because the reference might overlap
            // with what we're writing (e.g., for run-length encoding)
            let start = output.len() - offset;
            for j in 0..len {
                let byte = output[start + j];
                output.push(byte);
            }
        }
    }

    if output.len() != uncompressed_len {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            format!(
                "LZF: decompressed size {} doesn't match expected {}",
                output.len(),
                uncompressed_len
            ),
        )
        .into());
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompress_literal_only() {
        // Literal run of 5 bytes: "hello"
        let compressed = [0x04, b'h', b'e', b'l', b'l', b'o'];
        let result = decompress(&compressed, 5).unwrap();
        assert_eq!(result, b"hello");
    }

    #[test]
    fn test_decompress_with_backref() {
        // "abcabc" - first "abc" as literal, then back-reference
        // Literal: 0x02 (3 bytes), then "abc"
        // Backref: (1 << 5) | 0 = 0x20, offset low = 2 (offset 3, length 3)
        let compressed = [0x02, b'a', b'b', b'c', 0x20, 0x02];
        let result = decompress(&compressed, 6).unwrap();
        assert_eq!(result, b"abcabc");
    }

    #[test]
    fn test_decompress_run_length() {
        // "aaaaa" - 'a' followed by self-reference (run-length encoding)
        // Literal: 0x00 (1 byte), then 'a'
        // Backref: (1 << 5) | 0 = 0x20, offset_low = 0 (offset = 1 byte back), len = 1
        // + 2 = 3 This gives us "a" + "aaa" = "aaaa"
        let compressed = [0x00, b'a', 0x20, 0x00]; // literal 'a', then copy 3 from offset 1
        let result = decompress(&compressed, 4).unwrap();
        assert_eq!(result, b"aaaa");
    }

    #[test]
    fn test_decompress_empty() {
        let compressed: [u8; 0] = [];
        let result = decompress(&compressed, 0).unwrap();
        assert_eq!(result, [] as [u8; 0]);
    }

    #[test]
    fn test_decompress_long_literal() {
        // 31 bytes literal (max single literal run)
        let mut compressed = vec![0x1E]; // 30 + 1 = 31 bytes
        compressed.extend_from_slice(&[b'x'; 31]);
        let result = decompress(&compressed, 31).unwrap();
        assert_eq!(result, vec![b'x'; 31]);
    }

    #[test]
    fn test_decompress_size_mismatch() {
        let compressed = [0x02, b'a', b'b', b'c'];
        let result = decompress(&compressed, 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_decompress_invalid_backref() {
        // Back-reference that points before the start
        let compressed = [0x00, b'a', 0x20, 0x05]; // offset 6, but only 1 byte in output
        let result = decompress(&compressed, 4);
        assert!(result.is_err());
    }
}
