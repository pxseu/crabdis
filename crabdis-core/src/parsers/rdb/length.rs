use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Result of length decoding, which may indicate a special encoding.
#[derive(Debug, Clone, Copy)]
pub enum LengthResult {
    /// A plain length value.
    Length(u64),
    /// A special encoding type (INT8, INT16, INT32, LZF).
    Encoded(u8),
}

/// Loads a length-encoded value from the async reader.
///
/// Returns either a plain length or indicates a special encoding.
async fn load<R>(reader: &mut R) -> Result<LengthResult>
where
    R: AsyncRead + Unpin,
{
    let first_byte = reader.read_u8().await?;
    load_from_first_byte(reader, first_byte).await
}

/// Loads a length value, returning only the length (errors on special
/// encoding).
///
/// # Errors
///
/// Returns an error if the result is a special encoding, or I/O fails.
pub async fn load_plain<R>(reader: &mut R) -> Result<u64>
where
    R: AsyncRead + Unpin,
{
    match load(reader).await? {
        LengthResult::Length(len) => Ok(len),
        LengthResult::Encoded(enc) => Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Expected plain length, got encoding type: {enc}"),
        )
        .into()),
    }
}

/// Loads length from the first byte already read.
///
/// # Errors
///
/// Returns an error if the length encoding is invalid or I/O fails.
pub async fn load_from_first_byte<R>(reader: &mut R, first_byte: u8) -> Result<LengthResult>
where
    R: AsyncRead + Unpin,
{
    let encoding_type = (first_byte & 0xC0) >> 6;

    match encoding_type {
        super::symbols::LEN_6BIT => Ok(LengthResult::Length(u64::from(first_byte & 0x3F))),
        super::symbols::LEN_14BIT => {
            let next_byte = reader.read_u8().await?;
            Ok(LengthResult::Length(u64::from(
                (u16::from(first_byte & 0x3F)) << 8 | u16::from(next_byte),
            )))
        }
        super::symbols::LEN_32BIT => {
            // Check if it's actually a 64-bit length (first_byte == 0x81)
            if first_byte == 0x81 {
                let mut bytes = [0u8; 8];
                reader.read_exact(&mut bytes).await?;
                Ok(LengthResult::Length(u64::from_be_bytes(bytes)))
            } else {
                let mut bytes = [0u8; 4];
                reader.read_exact(&mut bytes).await?;
                Ok(LengthResult::Length(u64::from(u32::from_be_bytes(bytes))))
            }
        }
        super::symbols::LEN_ENCVAL => {
            // Special encoding - return the encoding type
            let enc_type = first_byte & 0x3F;
            Ok(LengthResult::Encoded(enc_type))
        }
        _ => Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Invalid length encoding: {encoding_type}"),
        )
        .into()),
    }
}

/// Saves a length-encoded value to the async writer.
///
/// Uses the most compact encoding possible:
/// - 6-bit for values 0-63
/// - 14-bit for values 64-16383
/// - 32-bit for larger values
///
/// # Errors
///
/// Returns an error if I/O fails.
pub async fn save<W>(writer: &mut W, len: u64) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    if len < 64 {
        // 6-bit encoding: 00xxxxxx
        #[allow(clippy::cast_possible_truncation)]
        writer.write_u8(len as u8).await?;
    } else if len < 16384 {
        // 14-bit encoding: 01xxxxxx xxxxxxxx
        #[allow(clippy::cast_possible_truncation)]
        let high = ((len >> 8) as u8) | 0x40;
        #[allow(clippy::cast_possible_truncation)]
        let low = len as u8;
        writer.write_all(&[high, low]).await?;
    } else {
        // 32-bit encoding: 10000000 + 4 bytes big-endian
        writer.write_u8(0x80).await?;
        #[allow(clippy::cast_possible_truncation)]
        writer.write_all(&(len as u32).to_be_bytes()).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_load_6bit() {
        let data = [0x05]; // 6-bit: 5
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Length(5)));
    }

    #[tokio::test]
    async fn test_load_6bit_max() {
        let data = [0x3F]; // 6-bit: 63 (max)
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Length(63)));
    }

    #[tokio::test]
    async fn test_load_14bit() {
        let data = [0x41, 0x00]; // 14-bit: 256
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Length(256)));
    }

    #[tokio::test]
    async fn test_load_14bit_max() {
        let data = [0x7F, 0xFF]; // 14-bit: 16383 (max)
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Length(16383)));
    }

    #[tokio::test]
    async fn test_load_32bit() {
        let data = [0x80, 0x00, 0x01, 0x00, 0x00]; // 32-bit: 65536
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Length(65536)));
    }

    #[tokio::test]
    async fn test_load_encoded_int8() {
        let data = [0xC0]; // ENCVAL + INT8
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Encoded(0)));
    }

    #[tokio::test]
    async fn test_load_encoded_int16() {
        let data = [0xC1]; // ENCVAL + INT16
        let mut reader = Cursor::new(&data);
        let result = load(&mut reader).await.unwrap();
        assert!(matches!(result, LengthResult::Encoded(1)));
    }

    #[tokio::test]
    async fn test_load_plain() {
        let data = [0x05];
        let mut reader = Cursor::new(&data);
        let len = load_plain(&mut reader).await.unwrap();
        assert_eq!(len, 5);
    }

    #[tokio::test]
    async fn test_load_plain_rejects_encoded() {
        let data = [0xC0]; // ENCVAL
        let mut reader = Cursor::new(&data);
        let result = load_plain(&mut reader).await;
        assert!(result.is_err());
    }
}
