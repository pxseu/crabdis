use crate::prelude::*;

/// Parses a signed integer from a reader.
///
/// The function reads the integer until the \r\n is encountered.
/// Uses `size::deserialize` as the base for parsing unsigned digits,
/// then applies the sign if present.
///
/// # Errors
///
/// Returns the original `Ok(i64)` value if successful, or an [`Error::Io`] if
/// it fails.
#[allow(clippy::cast_possible_wrap)] // possible, but we don't care about that here since target are 64bit
pub async fn deserialize<T>(reader: &mut T) -> Result<i64>
where
    T: AsyncRead + Unpin,
{
    let first = reader.read_u8().await?;

    if first == b'-' {
        let val = super::size::deserialize(reader).await?;
        Ok(-(val as i64))
    } else {
        let val = super::size::deserialize_first(first, reader).await?;
        Ok(val as i64)
    }
}

/// Serializes a signed integer to a writer.
///
/// Uses `size::serialize` as the base for the unsigned portion,
/// prepending `-` if negative.
///
/// # Errors
///
/// Returns the original `Ok(())` value if successful, or an [`Error::Io`] if it
/// fails.
#[allow(clippy::cast_possible_truncation)] // possible, but we don't care about that here since target are 64bit
pub async fn serialize<T>(writer: &mut T, value: i64) -> Result<()>
where
    T: AsyncWrite + Unpin + ?Sized,
{
    use tokio::io::AsyncWriteExt;

    if value.is_negative() {
        writer.write_u8(b'-').await?;
    }

    super::size::serialize(writer, value.unsigned_abs() as usize).await
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    async fn parse_from(bytes: &[u8]) -> Result<i64> {
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
    async fn test_parse_integer() {
        // there is a parrent that checks the first byte, so we don't need to do that
        // here
        let value = parse_from(b"42\r\n").await.unwrap();
        assert_eq!(value, 42);

        let value = parse_from(b"-100\r\n").await.unwrap();
        assert_eq!(value, -100);

        let value = parse_from(b"0\r\n").await.unwrap();
        assert_eq!(value, 0);

        let value = parse_from(b"1234567890\r\n").await.unwrap();
        assert_eq!(value, 1_234_567_890);

        let value = parse_from(b"-1\r\n").await.unwrap();
        assert_eq!(value, -1); // this is also NIL, but we don't care about that here
    }

    #[tokio::test]
    async fn rejects_empty_integer() {
        let err = parse_from(b"\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid size");
    }

    #[tokio::test]
    async fn rejects_minus_only() {
        let err = parse_from(b"-\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid size");
    }

    #[tokio::test]
    async fn rejects_minus_after_digits() {
        let err = parse_from(b"12-\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid size");
    }

    #[tokio::test]
    async fn rejects_garbage_in_number() {
        let err = parse_from(b"12x\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid size");
    }

    #[tokio::test]
    async fn consumes_all_of_crlf() {
        let mut reader = Cursor::new(b"5\r\nZ");
        let value = deserialize(&mut reader).await.unwrap();
        assert_eq!(value, 5);
        // next byte should be 'Z'
        assert_eq!(reader.read_u8().await.unwrap(), b'Z');
        assert!(reader.read_u8().await.is_err());
    }

    #[tokio::test]
    async fn test_serialize_integer() {
        let mut writer = Cursor::new(Vec::new());
        serialize(&mut writer, 42).await.unwrap();
        assert_eq!(writer.get_ref(), b"42\r\n");

        let mut writer = Cursor::new(Vec::new());
        serialize(&mut writer, -100).await.unwrap();
        assert_eq!(writer.get_ref(), b"-100\r\n");

        let mut writer = Cursor::new(Vec::new());
        serialize(&mut writer, 0).await.unwrap();
        assert_eq!(writer.get_ref(), b"0\r\n");
    }
}
