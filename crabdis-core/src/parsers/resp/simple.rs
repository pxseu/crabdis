use crate::prelude::*;

/// Reads a simple line until \r\n (after the type prefix was already consumed)
///
/// # Errors
///
/// Returns the original `Ok(Arc<str>)` value if successful, or an [`Error::Io`]
/// if it fails.
pub async fn deserialize<T>(reader: &mut T) -> Result<Arc<str>>
where
    T: AsyncRead + Unpin,
{
    // capacity is a guess, it will be resized if needed
    let mut buf = Vec::with_capacity(16);

    loop {
        match reader.read_u8().await? {
            b'\r' => {
                super::crlf::deserialize_lf(reader).await?;
                // SAFETY: RESP simple strings should be ASCII
                let s = unsafe { str::from_utf8_unchecked(&buf) };
                return Ok(Arc::from(s));
            }
            byte => buf.push(byte),
        }
    }
}

/// Serializes a simple string into a writer.
///
/// # Errors
///
/// Returns the original `Ok(())` value if successful, or an [`Error::Io`]
/// if it fails.
#[inline]
pub async fn serialize<T>(writer: &mut T, value: &str) -> Result<()>
where
    T: AsyncWrite + Unpin + ?Sized,
{
    writer.write_all(value.as_bytes()).await?;
    super::crlf::serialize(writer).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    async fn parse_from(bytes: &[u8]) -> Result<Arc<str>> {
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
    async fn test_simple_deserialize() {
        let cases = [
            (b"OK\r\n".as_ref(), "OK"),
            (b"hello world\r\n".as_ref(), "hello world"),
            (b"1234567890\r\n".as_ref(), "1234567890"),
            (b"\r\n".as_ref(), ""),
        ];

        for (input, expected) in cases {
            let result = parse_from(input).await.unwrap();
            assert_eq!(&*result, expected, "Failed for input: {input:?}");
        }
    }

    #[tokio::test]
    async fn test_simple_serialize() {
        let cases = [
            ("OK", b"OK\r\n".as_ref()),
            ("hello world", b"hello world\r\n".as_ref()),
            ("1234567890", b"1234567890\r\n".as_ref()),
            ("", b"\r\n".as_ref()),
        ];

        for (input, expected) in cases {
            let mut writer = Cursor::new(Vec::new());
            serialize(&mut writer, input).await.unwrap();
            let result = writer.into_inner();
            assert_eq!(&result[..], expected, "Failed for input: {input:?}");
        }
    }
}
