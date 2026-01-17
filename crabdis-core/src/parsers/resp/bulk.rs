use crate::prelude::*;

/// Deserializes a bulk string/error (after the `$` or `!` prefix was consumed).
///
/// Returns `None` if the length is -1 (NIL in RESP2).
///
/// # Errors
///
/// Returns an [`Error::Io`] if reading fails.
pub async fn deserialize<T>(reader: &mut T) -> Result<Option<Arc<str>>>
where
    T: AsyncRead + Unpin,
{
    let Some(len) = super::size::deserialize_nullable(reader).await? else {
        return Ok(None);
    };

    if len == 0 {
        super::crlf::deserialize(reader).await?;
        return Ok(Some(Arc::from("")));
    }

    let mut value = vec![0; len];
    reader.read_exact(&mut value).await?;

    // SAFETY: RESP strings should be valid UTF-8
    let value = unsafe { std::str::from_utf8_unchecked(&value) };

    super::crlf::deserialize(reader).await?;

    Ok(Some(Arc::from(value)))
}

/// Serializes a bulk string/error (without the prefix byte).
///
/// Writes: `<length>\r\n<data>\r\n`
///
/// # Errors
///
/// Returns an [`Error::Io`] if writing fails.
#[inline]
pub async fn serialize<T>(writer: &mut T, value: &str) -> Result<()>
where
    T: AsyncWrite + Unpin + ?Sized,
{
    super::size::serialize(writer, value.len()).await?;
    writer.write_all(value.as_bytes()).await?;
    super::crlf::serialize(writer).await
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    async fn parse_from(bytes: &[u8]) -> Result<Option<Arc<str>>> {
        let mut reader = Cursor::new(bytes);

        match deserialize(&mut reader).await {
            Ok(value) => {
                assert!(
                    reader.read_u8().await.is_err(),
                    "Reader should be empty, for buffer: {bytes:?} got {value:?}"
                );
                Ok(value)
            }
            Err(e) => Err(e),
        }
    }

    #[tokio::test]
    async fn test_bulk_deserialize() {
        let cases = [
            (b"5\r\nhello\r\n".as_ref(), Some("hello")),
            (b"0\r\n\r\n".as_ref(), Some("")),
            (b"13\r\nHello, World!\r\n".as_ref(), Some("Hello, World!")),
            (b"-1\r\n".as_ref(), None),
        ];

        for (input, expected) in cases {
            let result = parse_from(input).await.unwrap();
            assert_eq!(result.as_deref(), expected, "Failed for input: {input:?}");
        }
    }

    #[tokio::test]
    async fn test_bulk_serialize() {
        let cases = [
            ("hello", b"5\r\nhello\r\n".as_ref()),
            ("", b"0\r\n\r\n".as_ref()),
            ("Hello, World!", b"13\r\nHello, World!\r\n".as_ref()),
        ];

        for (input, expected) in cases {
            let mut writer = Cursor::new(Vec::new());
            serialize(&mut writer, input).await.unwrap();
            let result = writer.into_inner();
            assert_eq!(&result[..], expected, "Failed for input: {input:?}");
        }
    }

    #[tokio::test]
    async fn test_bulk_round_trip() {
        let values = ["test", "hello world", "with\nnewline", ""];

        for original in values {
            let mut buff = Vec::new();
            serialize(&mut buff, original).await.unwrap();

            let mut reader = Cursor::new(&buff);
            let parsed = deserialize(&mut reader).await.unwrap();

            assert_eq!(
                parsed.as_deref(),
                Some(original),
                "Round trip failed for {original:?}"
            );
        }
    }
}
