use std::io::{Error as IoError, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::prelude::*;

/// CRC64 polynomial used by Redis (Jones polynomial, reflected).
const CRC64_POLY: u64 = 0x95AC_9329_AC4B_C9B5;

/// Precomputed CRC64 lookup table (reflected implementation).
#[allow(clippy::cast_possible_truncation)]
static CRC64_TABLE: [u64; 256] = {
    let mut table = [0u64; 256];
    let mut i = 0u64;
    while i < 256 {
        let mut crc = i;
        let mut j = 0;
        while j < 8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ CRC64_POLY;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

/// Computes CRC64 checksum for a byte slice using Redis's algorithm.
#[cfg(test)]
#[must_use]
pub fn crc64(data: &[u8]) -> u64 {
    crc64_update(0, data)
}

/// Updates an existing CRC64 checksum with additional data.
#[must_use]
pub fn crc64_update(crc: u64, data: &[u8]) -> u64 {
    let mut crc = crc;
    for &byte in data {
        let index = ((crc ^ u64::from(byte)) & 0xFF) as usize;
        crc = CRC64_TABLE[index] ^ (crc >> 8);
    }
    crc
}

/// An async reader wrapper that computes CRC64 as data is read.
pub struct Crc64Reader<R> {
    inner: R,
    crc: u64,
}

impl<R> Crc64Reader<R> {
    /// Creates a new `Crc64Reader` wrapping the given reader.
    pub const fn new(inner: R) -> Self {
        Self { inner, crc: 0 }
    }

    /// Returns the current CRC64 checksum of all data read so far.
    #[cfg(test)]
    pub const fn checksum(&self) -> u64 {
        self.crc
    }

    /// Consumes the reader and returns the inner reader along with the final
    /// checksum.
    #[must_use]
    pub fn into_inner(self) -> (R, u64) {
        (self.inner, self.crc)
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for Crc64Reader<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before_len = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);

        if matches!(&result, Poll::Ready(Ok(()))) {
            let after_len = buf.filled().len();
            if after_len > before_len {
                let new_data = &buf.filled()[before_len..after_len];
                self.crc = crc64_update(self.crc, new_data);
            }
        }

        result
    }
}

/// An async writer wrapper that computes CRC64 as data is written.
pub struct Crc64Writer<W> {
    inner: W,
    crc: u64,
}

impl<W> Crc64Writer<W> {
    /// Creates a new `Crc64Writer` wrapping the given writer.
    pub const fn new(inner: W) -> Self {
        Self { inner, crc: 0 }
    }

    /// Returns the current CRC64 checksum of all data written so far.
    #[must_use]
    pub const fn checksum(&self) -> u64 {
        self.crc
    }

    /// Consumes the writer and returns the inner writer along with the final
    /// checksum.
    #[must_use]
    pub fn into_inner(self) -> (W, u64) {
        (self.inner, self.crc)
    }
}

impl<W: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for Crc64Writer<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let result = Pin::new(&mut self.inner).poll_write(cx, buf);

        if let Poll::Ready(Ok(n)) = &result {
            self.crc = crc64_update(self.crc, &buf[..*n]);
        }

        result
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Verifies that the computed checksum matches the stored checksum.
///
/// # Errors
///
/// Returns an error if the checksums don't match.
pub fn verify(computed: u64, stored: u64) -> Result<()> {
    if computed == stored {
        Ok(())
    } else {
        Err(IoError::new(
            ErrorKind::InvalidData,
            format!("CRC64 checksum mismatch: computed {computed:#018x}, stored {stored:#018x}"),
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tokio::io::AsyncReadExt;

    use super::*;

    #[test]
    fn test_crc64_empty() {
        assert_eq!(crc64(&[]), 0);
    }

    #[test]
    fn test_crc64_known_value() {
        // Test with "123456789" - known test vector for Redis CRC64
        let data = b"123456789";
        let checksum = crc64(data);
        // Redis CRC64 of "123456789" should be 0xe9c6d914c4b8d9ca
        assert_eq!(checksum, 0xE9C6_D914_C4B8_D9CA);
    }

    #[test]
    fn test_crc64_update() {
        let data = b"hello world";
        let full = crc64(data);

        // Split and update incrementally
        let part1 = crc64(b"hello ");
        let part2 = crc64_update(part1, b"world");

        assert_eq!(full, part2);
    }

    #[tokio::test]
    async fn test_crc64_reader() {
        let data = b"hello world";
        let expected = crc64(data);

        let cursor = Cursor::new(data);
        let mut reader = Crc64Reader::new(cursor);

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await.unwrap();

        assert_eq!(buf, data);
        assert_eq!(reader.checksum(), expected);
    }

    #[tokio::test]
    async fn test_crc64_reader_partial_reads() {
        let data = b"hello world";
        let expected = crc64(data);

        let cursor = Cursor::new(data);
        let mut reader = Crc64Reader::new(cursor);

        // Read in chunks
        let mut buf = [0u8; 3];
        let mut total = Vec::new();

        loop {
            let n = reader.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            total.extend_from_slice(&buf[..n]);
        }

        assert_eq!(total, data);
        assert_eq!(reader.checksum(), expected);
    }

    #[test]
    fn test_verify_success() {
        let checksum = crc64(b"test");
        assert!(verify(checksum, checksum).is_ok());
    }

    #[test]
    fn test_verify_failure() {
        let checksum = crc64(b"test");
        assert!(verify(checksum, checksum + 1).is_err());
    }
}
