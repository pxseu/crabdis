use std::io::{Error as IoError, ErrorKind};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use crate::prelude::*;

/// Parses an integer from a reader.
///
/// The function reads the integer until the \r\n is encountered.
/// The function returns the integer and the reader is advanced to the next byte
/// after the \r\n. It is recommended and expected to use a buffered reader for
/// this function, as it calls `read_u8()` multiple times. This function assumes
/// the first byte is already checked to be a digit from resp (simple, any bulk
/// operation etc)
///
/// # Errors
///
/// Returns the original `Ok(i64)` value if successful, or an [`Error::Io`] if it fails.
pub async fn deserialize<T>(reader: &mut T) -> Result<i64>
where
    T: AsyncRead + Unpin,
{
    let mut value: i64 = 0;
    let mut negative = false;
    let mut seen_digit = false;

    loop {
        match reader.read_u8().await? {
            b'-' if negative => {
                return Err(IoError::new(ErrorKind::InvalidData, "Invalid integer").into());
            }
            b'-' if !seen_digit => {
                negative = true;
            }
            b'\r' => {
                if !seen_digit {
                    return Err(IoError::new(ErrorKind::InvalidData, "Invalid integer").into());
                }

                // read the \n and drop it
                reader.read_exact(&mut [0; 1]).await?;
                return Ok(if negative { -value } else { value });
            }
            byte @ b'0'..=b'9' => {
                let digit = i64::from(byte - b'0');

                // inline the check for integer overflow, the speed is drastically faster than
                // the checked_mul/add
                if value > i64::MAX / 10 || (value == i64::MAX / 10 && digit > i64::MAX % 10) {
                    return Err(IoError::new(ErrorKind::InvalidData, "Integer too large").into());
                }
                value = value * 10 + digit;

                seen_digit = true;
            }
            _ => return Err(IoError::new(ErrorKind::InvalidData, "Invalid integer").into()),
        }
    }
}

/// eq to `i64::MAX.to_string().len()` + 2 (for CRLF) + 1 (for sign)
const INT_MAX_LEN: usize = 22;

/// Serializes an integer to a writer.
///
/// The function writes the integer to the writer.
/// The function assumes the writer is a buffered writer, as it calls `write_all()` multiple times.
/// The function appends the CRLF to the end of the integer.
///
/// # Errors
///
/// Returns the original `Ok(())` value if successful, or an [`Error::Io`] if it fails.
pub async fn serialize<T>(writer: &mut T, value: i64) -> Result<()>
where
    T: AsyncWrite + Unpin + ?Sized,
{
    // cheap buffer on the stack for the integer
    let mut buf = [0u8; INT_MAX_LEN];
    let mut idx = INT_MAX_LEN;

    // add CRLF
    idx -= 2;
    buf[idx..].copy_from_slice(b"\r\n");

    let negative = value < 0;
    let mut u = value.unsigned_abs();

    if u == 0 {
        idx -= 1;
        buf[idx] = b'0';

        writer.write_all(&buf[idx..]).await?;
        return Ok(());
    }

    while u > 0 {
        idx -= 1;
        buf[idx] = b'0' + (u % 10) as u8;
        u /= 10;
    }

    if negative {
        idx -= 1;
        buf[idx] = b'-';
    }

    writer.write_all(&buf[idx..]).await?;

    Ok(())
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
        assert_eq!(err.to_string(), "Invalid integer");
    }

    #[tokio::test]
    async fn rejects_minus_only() {
        let err = parse_from(b"-\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid integer");
    }

    #[tokio::test]
    async fn rejects_minus_after_digits() {
        let err = parse_from(b"12-\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid integer");
    }

    #[tokio::test]
    async fn rejects_garbage_in_number() {
        let err = parse_from(b"12x\r\n").await.unwrap_err();
        assert_eq!(err.to_string(), "Invalid integer");
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
