use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Deserializes an unsigned size/length value until `\r\n`.
///
/// This is the base parser for unsigned integers. Used by arrays, maps, sets,
/// push, and as a building block for signed integer parsing.
///
/// # Errors
///
/// Returns an [`Error::Io`] if reading fails or the value is invalid.
#[inline]
pub async fn deserialize<T>(reader: &mut T) -> Result<usize>
where
    T: AsyncRead + Unpin,
{
    let first = reader.read_u8().await?;
    deserialize_first(first, reader).await
}

/// Deserializes an unsigned size/length value when the first byte was already
/// read.
///
/// # Errors
///
/// Returns an [`Error::Io`] if reading fails or the value is invalid.
pub async fn deserialize_first<T>(first: u8, reader: &mut T) -> Result<usize>
where
    T: AsyncRead + Unpin,
{
    if !first.is_ascii_digit() {
        return Err(IoError::from(ErrorKind::InvalidData).into());
    }

    let mut value = first.wrapping_sub(b'0') as usize;

    loop {
        match reader.read_u8().await? {
            b'\r' => {
                super::crlf::deserialize_lf(reader).await?;
                return Ok(value);
            }
            byte @ b'0'..=b'9' => {
                let digit = (byte - b'0') as usize;

                // overflow check
                if value > usize::MAX / 10 || (value == usize::MAX / 10 && digit > usize::MAX % 10)
                {
                    return Err(IoError::new(ErrorKind::InvalidData, "Size too large").into());
                }
                value = value * 10 + digit;
            }
            _ => return Err(IoError::from(ErrorKind::InvalidData).into()),
        }
    }
}

/// Deserializes a size that can be -1 (NIL).
///
/// Returns `Some(len)` for non-negative values, `None` for -1.
/// Errors on other negative values.
///
/// Used by bulk strings where -1 indicates NIL.
///
/// # Errors
///
/// Returns an [`Error::Io`] if reading fails or the value is invalid.
pub async fn deserialize_nullable<T>(reader: &mut T) -> Result<Option<usize>>
where
    T: AsyncRead + Unpin,
{
    let first = reader.read_u8().await?;

    if first == b'-' {
        // Must be -1 for NIL
        let val = deserialize(reader).await?;
        if val == 1 {
            return Ok(None);
        }
        return Err(IoError::from(ErrorKind::InvalidData).into());
    }

    let val = deserialize_first(first, reader).await?;
    Ok(Some(val))
}

/// eq to `log10(usize::MAX)` on 64-bit = 19.27 floored to 19, +1 for rounding,
const SIZE_MAX_LEN: usize = usize::MAX.ilog10() as usize + 1;
// +2 for CRLF
const SIZE_BUFFER_INIT: [u8; SIZE_MAX_LEN + 2] = {
    let mut buf = [0u8; SIZE_MAX_LEN + 2];
    buf[SIZE_MAX_LEN] = b'\r';
    buf[SIZE_MAX_LEN + 1] = b'\n';
    buf
};

/// Serializes an unsigned size/length value.
///
/// Writes: `<digits>\r\n`
///
/// This is the base serializer for unsigned integers.
///
/// # Errors
///
/// Returns an [`Error::Io`] if writing fails.
#[inline]
#[allow(clippy::cast_possible_truncation)] // v is always less than 10 so within the range of u8
pub async fn serialize<T>(writer: &mut T, value: usize) -> Result<()>
where
    T: AsyncWrite + Unpin + ?Sized,
{
    let mut buf = SIZE_BUFFER_INIT;
    let mut idx = SIZE_MAX_LEN;

    let mut v = value;
    loop {
        idx -= 1;
        buf[idx] = b'0' + (v % 10) as u8;
        v /= 10;

        if v == 0 {
            writer.write_all(&buf[idx..]).await?;
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tokio::io::AsyncReadExt;

    use super::*;

    async fn parse_from(bytes: &[u8]) -> Result<usize> {
        let mut reader = Cursor::new(bytes);

        match deserialize(&mut reader).await {
            Ok(value) => {
                assert!(
                    reader.read_u8().await.is_err(),
                    "Reader should be empty, for buffer: {bytes:?} got {value}"
                );
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }

    #[tokio::test]
    async fn test_deserialize() {
        let cases = [
            (b"0\r\n".as_ref(), 0usize),
            (b"1\r\n".as_ref(), 1),
            (b"42\r\n".as_ref(), 42),
            (b"1000\r\n".as_ref(), 1000),
            (b"1234567890\r\n".as_ref(), 1_234_567_890),
        ];

        for (input, expected) in cases {
            let result = parse_from(input).await.unwrap();
            assert_eq!(result, expected, "Failed for input: {input:?}");
        }
    }

    #[tokio::test]
    async fn test_deserialize_rejects_negative() {
        // `-` is not a valid digit, so it errors
        let err = parse_from(b"-1\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "invalid data");
    }

    #[tokio::test]
    async fn test_deserialize_rejects_empty() {
        let err = parse_from(b"\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "invalid data");
    }

    #[tokio::test]
    async fn test_deserialize_rejects_garbage() {
        let err = parse_from(b"12x\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "invalid data");
    }

    #[tokio::test]
    async fn test_deserialize_first() {
        // Test with pre-read first byte
        let mut reader = Cursor::new(b"2\r\nZ");
        let result = deserialize_first(b'4', &mut reader).await.unwrap();
        assert_eq!(result, 42);
        // next byte should be 'Z'
        assert_eq!(reader.read_u8().await.unwrap(), b'Z');
    }

    #[tokio::test]
    async fn test_deserialize_nullable() {
        let cases = [
            (b"0\r\n".as_ref(), Some(0usize)),
            (b"42\r\n".as_ref(), Some(42)),
            (b"-1\r\n".as_ref(), None),
        ];

        for (input, expected) in cases {
            let mut reader = Cursor::new(input);
            let result = deserialize_nullable(&mut reader).await.unwrap();
            assert_eq!(result, expected, "Failed for input: {input:?}");
        }
    }

    #[tokio::test]
    async fn test_deserialize_nullable_rejects_other_negatives() {
        let mut reader = Cursor::new(b"-2\r\n");
        let err = deserialize_nullable(&mut reader).await.unwrap_err();
        assert_eq!(err.to_string(), "invalid data");
    }

    #[tokio::test]
    async fn test_serialize() {
        let cases = [
            (0usize, b"0\r\n".as_ref()),
            (1, b"1\r\n".as_ref()),
            (42, b"42\r\n".as_ref()),
            (1000, b"1000\r\n".as_ref()),
        ];

        for (input, expected) in cases {
            let mut writer = Cursor::new(Vec::new());
            serialize(&mut writer, input).await.unwrap();
            let result = writer.into_inner();
            assert_eq!(&result[..], expected, "Failed for input: {input}");
        }
    }

    #[tokio::test]
    async fn test_round_trip() {
        let values = [0usize, 1, 42, 1000, 999_999];

        for original in values {
            let mut buff: Vec<u8> = Vec::new();
            serialize(&mut buff, original).await.unwrap();

            let mut reader = Cursor::new(&buff);
            let parsed = deserialize(&mut reader).await.unwrap();

            assert_eq!(original, parsed, "Round trip failed for {original}");
        }
    }
}
