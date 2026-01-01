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
                reader.read_exact(&mut [0; 1]).await?;
                // SAFETY: RESP simple strings should be ASCII
                let s = unsafe { String::from_utf8_unchecked(buf) };
                return Ok(Arc::from(s));
            }
            byte => buf.push(byte),
        }
    }
}
