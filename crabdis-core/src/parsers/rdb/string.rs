use std::io::{Error as IoError, ErrorKind};

use super::length::LengthResult;
use crate::prelude::*;

/// Loads a string from the async reader.
///
/// Handles plain strings and integer-encoded strings.
///
/// # Errors
///
/// Returns an error if string encoding is invalid or I/O fails.
pub async fn load<R>(reader: &mut R) -> Result<Arc<str>>
where
    R: AsyncRead + Unpin,
{
    let first_byte = reader.read_u8().await?;
    let result = super::length::load_from_first_byte(reader, first_byte).await?;

    match result {
        LengthResult::Length(len) => {
            #[allow(clippy::cast_possible_truncation)]
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf).await?;

            // Use from_utf8_lossy to handle potentially binary data gracefully
            let s = String::from_utf8_lossy(&buf).into_owned();
            Ok(s.into())
        }
        LengthResult::Encoded(enc_type) => load_encoded(reader, enc_type).await,
    }
}

/// Loads an encoded string (integer or LZF compressed).
async fn load_encoded<R>(reader: &mut R, enc_type: u8) -> Result<Arc<str>>
where
    R: AsyncRead + Unpin,
{
    match enc_type {
        super::symbols::ENC_INT8 => {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf).await?;
            let val = i8::from_le_bytes(buf);
            Ok(val.to_string().into())
        }
        super::symbols::ENC_INT16 => {
            let mut bytes = [0u8; 2];
            reader.read_exact(&mut bytes).await?;
            let val = i16::from_le_bytes(bytes);
            Ok(val.to_string().into())
        }
        super::symbols::ENC_INT32 => {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes).await?;
            let val = i32::from_le_bytes(bytes);
            Ok(val.to_string().into())
        }
        super::symbols::ENC_LZF => {
            // LZF compressed string
            let compressed_len = super::length::load_plain(reader).await?;
            let uncompressed_len = super::length::load_plain(reader).await?;

            #[allow(clippy::cast_possible_truncation)]
            let mut compressed = vec![0u8; compressed_len as usize];
            reader.read_exact(&mut compressed).await?;

            #[allow(clippy::cast_possible_truncation)]
            let decompressed = super::lzf::decompress(&compressed, uncompressed_len as usize)?;

            let s = String::from_utf8_lossy(&decompressed).into_owned();
            Ok(s.into())
        }
        _ => Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Unknown string encoding: {enc_type}"),
        )
        .into()),
    }
}

/// Loads raw bytes from the async reader (for encoded structures like
/// ziplists).
///
/// # Errors
///
/// Returns an error if encoding is invalid or I/O fails.
pub async fn load_raw<R>(reader: &mut R) -> Result<Vec<u8>>
where
    R: AsyncRead + Unpin,
{
    let first_byte = reader.read_u8().await?;
    let result = super::length::load_from_first_byte(reader, first_byte).await?;

    match result {
        LengthResult::Length(len) => {
            #[allow(clippy::cast_possible_truncation)]
            let mut buf = vec![0u8; len as usize];
            reader.read_exact(&mut buf).await?;
            Ok(buf)
        }
        LengthResult::Encoded(enc_type) => {
            match enc_type {
                super::symbols::ENC_LZF => {
                    // Read and decompress LZF data
                    let compressed_len = super::length::load_plain(reader).await?;
                    let uncompressed_len = super::length::load_plain(reader).await?;

                    #[allow(clippy::cast_possible_truncation)]
                    let mut compressed = vec![0u8; compressed_len as usize];
                    reader.read_exact(&mut compressed).await?;

                    #[allow(clippy::cast_possible_truncation)]
                    let decompressed =
                        super::lzf::decompress(&compressed, uncompressed_len as usize)?;
                    Ok(decompressed)
                }
                _ => {
                    // For integer encodings, return the first byte
                    // (this is a fallback, shouldn't happen for raw data)
                    Ok(vec![first_byte])
                }
            }
        }
    }
}

/// Saves a string to the async writer.
///
/// Uses LZF compression for strings longer than `MIN_COMPRESS_LEN` if
/// compression reduces size. Otherwise uses plain length-prefixed encoding.
///
/// # Errors
///
/// Returns an error if I/O fails.
#[allow(clippy::cast_possible_truncation)]
pub async fn save<W>(writer: &mut W, s: &str) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let bytes = s.as_bytes();

    // Try compression for strings above threshold
    if bytes.len() >= super::lzf::MIN_COMPRESS_LEN
        && let Some(compressed) = super::lzf::compress(bytes)
    {
        // Write LZF encoding marker
        writer.write_u8(0xC0 | super::symbols::ENC_LZF).await?;
        // Write compressed length
        super::length::save(writer, compressed.len() as u64).await?;
        // Write uncompressed length
        super::length::save(writer, bytes.len() as u64).await?;
        // Write compressed data
        writer.write_all(&compressed).await?;
        return Ok(());
    }

    // Fall back to plain encoding
    super::length::save(writer, bytes.len() as u64).await?;
    writer.write_all(bytes).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_load_plain_string() {
        let data = [0x05, b'h', b'e', b'l', b'l', b'o'];
        let mut reader = Cursor::new(&data);
        let s = load(&mut reader).await.unwrap();
        assert_eq!(s.as_ref(), "hello");
    }

    #[tokio::test]
    async fn test_load_empty_string() {
        let data = [0x00];
        let mut reader = Cursor::new(&data);
        let s = load(&mut reader).await.unwrap();
        assert_eq!(s.as_ref(), "");
    }

    #[tokio::test]
    async fn test_load_int8_encoded() {
        // String encoded as 8-bit integer: value 42
        let data = [0xC0, 42]; // ENCVAL + INT8
        let mut reader = Cursor::new(&data);
        let s = load(&mut reader).await.unwrap();
        assert_eq!(s.as_ref(), "42");
    }

    #[tokio::test]
    async fn test_load_int8_negative() {
        // String encoded as 8-bit integer: value -1
        let data = [0xC0, 0xFF]; // ENCVAL + INT8, -1 in two's complement
        let mut reader = Cursor::new(&data);
        let s = load(&mut reader).await.unwrap();
        assert_eq!(s.as_ref(), "-1");
    }

    #[tokio::test]
    async fn test_load_int16_encoded() {
        // String encoded as 16-bit integer: value 1000
        let data = [0xC1, 0xE8, 0x03]; // ENCVAL + INT16 + 1000 (little-endian)
        let mut reader = Cursor::new(&data);
        let s = load(&mut reader).await.unwrap();
        assert_eq!(s.as_ref(), "1000");
    }

    #[tokio::test]
    async fn test_load_int32_encoded() {
        // String encoded as 32-bit integer: value 100000
        let data = [0xC2, 0xA0, 0x86, 0x01, 0x00]; // ENCVAL + INT32 + 100000 (little-endian)
        let mut reader = Cursor::new(&data);
        let s = load(&mut reader).await.unwrap();
        assert_eq!(s.as_ref(), "100000");
    }

    #[tokio::test]
    async fn test_load_raw_bytes() {
        let data = [0x05, 0x01, 0x02, 0x03, 0x04, 0x05];
        let mut reader = Cursor::new(&data);
        let bytes = load_raw(&mut reader).await.unwrap();
        assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
    }
}
