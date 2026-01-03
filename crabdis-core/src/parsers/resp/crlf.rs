use std::io::{Error as IoError, ErrorKind};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::prelude::*;

/// Deserializes (consumes and validates) a CRLF sequence.
///
/// Checks CR first, only reads LF if CR is valid.
///
/// # Errors
///
/// Returns an [`Error::Io`] if reading fails or the bytes are not CRLF.
#[inline]
pub async fn deserialize<T>(reader: &mut T) -> Result<()>
where
    T: AsyncRead + Unpin,
{
    if reader.read_u8().await? != b'\r' {
        return Err(IoError::new(ErrorKind::InvalidData, "Expected CR").into());
    }

    if reader.read_u8().await? != b'\n' {
        return Err(IoError::new(ErrorKind::InvalidData, "Expected LF").into());
    }

    Ok(())
}

/// Deserializes (consumes and validates) just the LF part.
///
/// Use this when CR was already consumed as part of parsing logic.
///
/// # Errors
///
/// Returns an [`Error::Io`] if reading fails or the byte is not LF.
#[inline]
pub async fn deserialize_lf<T>(reader: &mut T) -> Result<()>
where
    T: AsyncRead + Unpin,
{
    if reader.read_u8().await? != b'\n' {
        return Err(IoError::new(ErrorKind::InvalidData, "Expected LF").into());
    }

    Ok(())
}

/// Serializes a CRLF sequence.
///
/// # Errors
///
/// Returns an [`Error::Io`] if writing fails.
#[inline]
pub async fn serialize<T>(writer: &mut T) -> Result<()>
where
    T: AsyncWrite + Unpin + ?Sized,
{
    writer.write_all(b"\r\n").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_deserialize_valid() {
        let mut reader = Cursor::new(b"\r\n");
        deserialize(&mut reader).await.unwrap();
    }

    #[tokio::test]
    async fn test_deserialize_rejects_wrong_cr() {
        let mut reader = Cursor::new(b"\n\n");
        let err = deserialize(&mut reader).await.unwrap_err();
        assert_eq!(err.to_string(), "Expected CR");
    }

    #[tokio::test]
    async fn test_deserialize_rejects_wrong_lf() {
        let mut reader = Cursor::new(b"\r\r");
        let err = deserialize(&mut reader).await.unwrap_err();
        assert_eq!(err.to_string(), "Expected LF");
    }

    #[tokio::test]
    async fn test_deserialize_lf_valid() {
        let mut reader = Cursor::new(b"\n");
        deserialize_lf(&mut reader).await.unwrap();
    }

    #[tokio::test]
    async fn test_deserialize_lf_rejects_wrong() {
        let mut reader = Cursor::new(b"\r");
        let err = deserialize_lf(&mut reader).await.unwrap_err();
        assert_eq!(err.to_string(), "Expected LF");
    }

    #[tokio::test]
    async fn test_serialize() {
        let mut writer = Cursor::new(Vec::new());
        serialize(&mut writer).await.unwrap();
        assert_eq!(writer.into_inner(), b"\r\n");
    }
}
