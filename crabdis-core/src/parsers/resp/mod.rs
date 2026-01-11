pub mod bulk;
pub mod crlf;
pub mod int;
pub mod simple;
pub mod size;
pub mod symbols;

use std::collections::{HashMap, HashSet};
use std::hint::unreachable_unchecked;
use std::pin::Pin;

use crate::prelude::*;
use crate::value::Value;

/// RESP parser implementation.
///
/// It is a singleton struct that can be used to parse RESP messages.
pub struct Resp;

impl Resp {
    /// # Errors
    ///
    /// Returns the original `Ok(true)` value if the reader is not empty, or
    /// `Ok(false)` if the reader is empty. Returns an [`Error::Io`] if the
    /// reader fails to fill the buffer.
    async fn can_read<R>(reader: &mut R) -> Result<bool>
    where
        R: AsyncBufRead + Unpin,
    {
        Ok(!reader.fill_buf().await?.is_empty())
    }

    /// # Errors
    ///
    /// Returns the original `Ok(Some(Value))` value if successful, or
    /// `Ok(None)` if the reader is empty. Returns an [`Error::Io`] if the
    /// reader fails to fill the buffer.
    pub async fn try_parse<'a, R>(
        reader: &'a mut R,
        version: u8,
    ) -> Result<Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send + 'a>>>
    where
        R: AsyncBufRead + Unpin + Send + 'a,
    {
        if !Self::can_read(reader).await? {
            return Ok(Box::pin(async move { Ok(None) }));
        }

        Ok(match version {
            2 => Self::from2(reader),
            3 => Self::from3(reader),
            _ => unreachable!("Invalid protocol version"),
        })
    }

    pub fn to2<'b, T>(
        value: &'b Value,
        writer: &'b mut T,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'b>>
    where
        T: AsyncWrite + Unpin + Send + 'b + ?Sized,
    {
        Box::pin(async move {
            match value {
                Value::Ok => Self::to2(&Value::Simple("OK".into()), writer).await,
                Value::Pong => Self::to2(&Value::Simple("PONG".into()), writer).await,
                Value::Nil => Ok(writer.write_all(b"$-1\r\n").await?),
                Value::Simple(s) => {
                    writer.write_u8(self::symbols::SIMPLE).await?;
                    self::simple::serialize(writer, s).await?;

                    Ok(())
                }
                Value::Error(e) => {
                    writer.write_u8(self::symbols::ERROR).await?;
                    self::simple::serialize(writer, e).await?;

                    Ok(())
                }
                Value::Integer(i) => {
                    writer.write_u8(self::symbols::INTEGER).await?;
                    self::int::serialize(writer, *i).await?;

                    Ok(())
                }
                Value::String(s) if s.is_empty() => Ok(writer.write_all(b"$-1\r\n").await?),
                Value::String(s) => {
                    writer.write_u8(self::symbols::BULK).await?;
                    self::bulk::serialize(writer, s).await?;

                    Ok(())
                }
                Value::Multi(v) | Value::Push(v) => {
                    writer.write_u8(self::symbols::ARRAY).await?;
                    self::size::serialize(writer, v.len()).await?;

                    for value in v.iter() {
                        Self::to2(value, writer).await?;
                    }

                    Ok(())
                }
                Value::Map(h) => {
                    writer.write_u8(self::symbols::ARRAY).await?;
                    // map in non resp3 is serialized as a list of key-value pairs
                    self::size::serialize(writer, h.len() * 2).await?;

                    for (k, v) in h {
                        Self::to2(k, writer).await?;
                        Self::to2(v, writer).await?;
                    }

                    Ok(())
                }
                Value::Set(s) => {
                    writer.write_u8(self::symbols::ARRAY).await?;
                    self::size::serialize(writer, s.len()).await?;

                    for v in s {
                        Self::to2(v, writer).await?;
                    }

                    Ok(())
                }

                Value::Expire(_) if Value::expired(value) => Self::to2(&Value::Nil, writer).await,
                Value::Expire((v, _)) => Self::to2(v, writer).await,
            }
        })
    }

    pub fn to3<'b, T>(
        value: &'b Value,
        writer: &'b mut T,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'b>>
    where
        T: AsyncWrite + Unpin + Send + 'b + ?Sized,
    {
        Box::pin(async move {
            match value {
                Value::Nil => Ok(writer.write_all(b"_\r\n").await?),
                Value::Map(map) => {
                    writer.write_u8(self::symbols::MAP).await?;
                    self::size::serialize(writer, map.len()).await?;

                    for (k, v) in map {
                        Self::to3(k, writer).await?;
                        Self::to3(v, writer).await?;
                    }

                    Ok(())
                }

                Value::Set(set) => {
                    writer.write_u8(self::symbols::SET).await?;
                    self::size::serialize(writer, set.len()).await?;

                    for v in set {
                        Self::to3(v, writer).await?;
                    }

                    Ok(())
                }

                Value::Error(s) => {
                    writer.write_u8(self::symbols::BULK_ERROR).await?;
                    self::bulk::serialize(writer, s).await?;

                    Ok(())
                }

                Value::Push(v) => {
                    writer.write_u8(self::symbols::PUSH).await?;
                    self::size::serialize(writer, v.len()).await?;

                    for value in v.iter() {
                        Self::to3(value, writer).await?;
                    }

                    Ok(())
                }

                Value::Expire(_) if value.expired() => Self::to3(&Value::Nil, writer).await,
                Value::Expire((v, _)) => Self::to3(v, writer).await,

                // rest of the code is the same as to_resp2
                _ => Self::to2(value, writer).await,
            }
        })
    }

    pub fn from2<'a, T>(
        reader: &'a mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send + 'a>>
    where
        T: AsyncRead + Unpin + Send + 'a,
    {
        Self::from2_impl(reader, None)
    }

    fn from2_impl<'a, T>(
        reader: &'a mut T,
        preread_byte: Option<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send + 'a>>
    where
        T: AsyncRead + Unpin + Send + 'a,
    {
        Box::pin(async move {
            let first_byte = match preread_byte {
                Some(b) => b,
                None => match reader.read_u8().await {
                    Ok(byte) => byte,
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                    Err(e) => return Err(e.into()),
                },
            };

            match first_byte {
                self::symbols::BULK => {
                    let value = self::bulk::deserialize(reader).await?;

                    value.map_or(Ok(Some(Value::Nil)), |s| Ok(Some(Value::String(s))))
                }

                self::symbols::INTEGER => {
                    let value = self::int::deserialize(reader).await?;

                    Ok(Some(Value::Integer(value)))
                }

                self::symbols::ARRAY => {
                    let len = self::size::deserialize(reader).await?;

                    if len == 0 {
                        return Ok(Some(Value::Multi([].into())));
                    }

                    let mut values = Vec::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from2(reader).await?;

                        if let Some(value) = value {
                            values.push(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(Value::Multi(values.into())))
                }

                self::symbols::MAP => {
                    let len = self::size::deserialize(reader).await?;

                    if len == 0 {
                        return Ok(Some(Value::Map(HashMap::with_capacity(0))));
                    }

                    let mut map = HashMap::with_capacity(len);

                    for _ in 0..len {
                        let key = Self::from2(reader).await?;
                        let value = Self::from2(reader).await?;

                        match (key, value) {
                            (Some(key), Some(value)) => {
                                map.insert(key, value);
                            }
                            _ => return Ok(None),
                        }
                    }

                    Ok(Some(Value::Map(map)))
                }

                self::symbols::SIMPLE => {
                    let value = self::simple::deserialize(reader).await?;

                    match value.as_ref() {
                        "OK" => Ok(Some(Value::Ok)),
                        "PONG" => Ok(Some(Value::Pong)),
                        v => Ok(Some(Value::Simple(v.into()))),
                    }
                }

                self::symbols::ERROR => {
                    let value = self::simple::deserialize(reader).await?;

                    Ok(Some(Value::Error(value)))
                }

                _ => Ok(Some(Value::Error("Invalid request".into()))),
            }
        })
    }

    pub fn from3<'a, T>(
        reader: &'a mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send + 'a>>
    where
        T: AsyncRead + Unpin + Send + 'a,
    {
        Box::pin(async move {
            let first_byte = match reader.read_u8().await {
                Ok(byte) => byte,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(e.into()),
            };

            match first_byte {
                // apart from bulk array i dont think any of these can be sent to a server
                self::symbols::PUSH | self::symbols::ARRAY | self::symbols::SET => {
                    let len = self::size::deserialize(reader).await?;

                    let mut values = Vec::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from3(reader).await?;

                        if let Some(value) = value {
                            values.push(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(match first_byte {
                        self::symbols::PUSH => Value::Push(values.into()),
                        self::symbols::ARRAY => Value::Multi(values.into()),
                        self::symbols::SET => {
                            // this is basically 1:1 copy of the code that is using .extend
                            let mut set = HashSet::with_capacity(len);

                            // force into_iter to free the values vector after the loop
                            for v in values {
                                // this should be as free as possible, it's only moving Arc's or
                                // integers, so 8 bytes max
                                set.insert(v);
                            }

                            Value::Set(set)
                        }
                        _ => unsafe { unreachable_unchecked() },
                    }))
                }

                // special nil symbol in resp3
                self::symbols::NIL => {
                    self::crlf::deserialize(reader).await?;

                    Ok(Some(Value::Nil))
                }

                _ => Self::from2_impl(reader, Some(first_byte)).await,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use tokio::time::Instant;

    use super::*;
    use crate::{value_multi, value_push};

    #[tokio::test]
    async fn test_resp2_can_read() {
        let mut reader = Cursor::new(b"*3\r\n$5\r\nhello\r\n:42\r\n$-1\r\n");
        assert!(Resp::can_read(&mut reader).await.expect("Failed to read"));

        let mut reader = Cursor::new(b"");
        assert!(!Resp::can_read(&mut reader).await.expect("Failed to read"));
    }

    #[tokio::test]
    async fn test_resp2_try_parse() {
        let mut reader = Cursor::new(b"*3\r\n$5\r\nhello\r\n:42\r\n$-1\r\n");
        let value = Resp::try_parse(&mut reader, 2)
            .await
            .expect("Failed to parse")
            .await
            .expect("Failed to await");
        assert_eq!(
            value,
            Some(value_multi![
                Value::String("hello".into()),
                Value::Integer(42),
                Value::Nil
            ])
        );
    }

    #[tokio::test]
    async fn test_resp2_try_parse_empty() {
        let mut reader = Cursor::new(b"");
        let value = Resp::try_parse(&mut reader, 2)
            .await
            .expect("Failed to parse")
            .await
            .expect("Failed to await");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_resp2_simple_string() {
        let value = Value::Simple("OK".into());
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"+OK\r\n");
    }

    #[tokio::test]
    async fn test_resp2_error() {
        let value = Value::Error("ERR something went wrong".into());
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"-ERR something went wrong\r\n");
    }

    #[tokio::test]
    async fn test_resp2_integer() {
        let value = Value::Integer(42);
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b":42\r\n");

        let value = Value::Integer(-100);
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b":-100\r\n");

        let value = Value::Integer(0);
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b":0\r\n");
    }

    #[tokio::test]
    async fn test_resp2_bulk_string() {
        let value = Value::String("Hello, World!".into());
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"$13\r\nHello, World!\r\n");

        // Empty string
        let value = Value::String("".into());
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"$-1\r\n");
    }

    #[tokio::test]
    async fn test_resp2_nil() {
        let value = Value::Nil;
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"$-1\r\n");
    }

    #[tokio::test]
    async fn test_resp2_array() {
        let value = Value::Multi(
            Vec::from([
                Value::String("Hello, World!".into()),
                Value::Integer(42),
                Value::Nil,
            ])
            .into(),
        );
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"*3\r\n$13\r\nHello, World!\r\n:42\r\n$-1\r\n");

        // Empty array
        let value = Value::Multi(Vec::new().into());
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"*0\r\n");
    }

    #[tokio::test]
    async fn test_resp2_map() {
        let mut map = HashMap::new();
        map.insert(Value::String("key1".into()), Value::String("value1".into()));
        map.insert(Value::String("key2".into()), Value::Integer(42));

        let value = Value::Map(map);
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();

        // Map serializes as array of key-value pairs in RESP2
        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with("*4\r\n")); // 2 keys * 2 = 4 elements
        assert!(resp.contains("$4\r\nkey1\r\n"));
        assert!(resp.contains("$6\r\nvalue1\r\n"));
    }

    #[tokio::test]
    async fn test_resp2_set() {
        let mut set = HashSet::new();
        set.insert(Value::String("a".into()));
        set.insert(Value::String("b".into()));
        set.insert(Value::Integer(42));

        let value = Value::Set(set);
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with("*3\r\n"));
    }

    #[tokio::test]
    async fn test_resp2_ok_pong() {
        let value = Value::Ok;
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"+OK\r\n");

        let value = Value::Pong;
        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"+PONG\r\n");
    }

    #[tokio::test]
    async fn test_resp3_nil() {
        let value = Value::Nil;
        let mut buff = Vec::new();
        Resp::to3(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"_\r\n");
    }

    #[tokio::test]
    async fn test_resp3_map() {
        let mut map = HashMap::new();
        map.insert(Value::String("key".into()), Value::String("value".into()));

        let value = Value::Map(map);
        let mut buff = Vec::new();
        Resp::to3(&value, &mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with("%1\r\n")); // % indicates map in RESP3
        assert!(resp.contains("$3\r\nkey\r\n"));
        assert!(resp.contains("$5\r\nvalue\r\n"));
    }

    #[tokio::test]
    async fn test_resp3_set() {
        let mut set = HashSet::new();
        set.insert(Value::String("a".into()));
        set.insert(Value::Integer(1));

        let value = Value::Set(set);
        let mut buff = Vec::new();
        Resp::to3(&value, &mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with("~2\r\n")); // ~ indicates set in RESP3
    }

    #[tokio::test]
    async fn test_resp3_error() {
        let value = Value::Error("ERR test error".into());
        let mut buff = Vec::new();
        Resp::to3(&value, &mut buff).await.unwrap();
        assert_eq!(buff, b"!14\r\nERR test error\r\n");
    }

    #[tokio::test]
    async fn test_resp3_push() {
        let value = value_push!(
            Value::String("message".into()),
            Value::String("channel".into())
        );
        let mut buff = Vec::new();
        Resp::to3(&value, &mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with(">2\r\n")); // > indicates push in RESP3
    }

    #[tokio::test]
    async fn test_parse_simple_string() {
        let mut reader = b"+OK\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Ok));

        let mut reader = b"+PONG\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Pong));
    }

    #[tokio::test]
    async fn test_parse_error() {
        let mut reader = b"-ERR unknown command\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Error("ERR unknown command".into())));
    }

    #[tokio::test]
    async fn test_parse_integer() {
        let mut reader = b":42\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(42)));

        let mut reader = b":-100\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(-100)));
    }

    #[tokio::test]
    async fn test_parse_bulk_string() {
        let buff = b"$5\r\nhello\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::String("hello".into())));

        let buff = b"$-1\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Nil));
    }

    #[tokio::test]
    async fn test_parse_array() {
        let mut reader = b"*3\r\n$5\r\nhello\r\n:42\r\n$-1\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(value_multi![
                Value::String("hello".into()),
                Value::Integer(42),
                Value::Nil
            ]),
        );

        let mut reader = b"*0\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(value_multi![]));
    }

    #[tokio::test]
    async fn test_parse_map() {
        let mut reader = b"%2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n$4\r\nkey2\r\n:42\r\n".as_ref();
        let value = Resp::from2(&mut reader).await.unwrap();

        if let Some(Value::Map(map)) = value {
            assert_eq!(map.len(), 2);
            assert_eq!(
                map.get(&Value::String("key".into())),
                Some(&Value::String("value".into()))
            );
            assert_eq!(
                map.get(&Value::String("key2".into())),
                Some(&Value::Integer(42))
            );
        } else {
            panic!("Expected Map value");
        }
    }

    #[tokio::test]
    async fn test_parse_set() {
        let mut reader = b"~3\r\n$1\r\na\r\n$1\r\nb\r\n:1\r\n".as_ref();
        let value = Resp::from3(&mut reader).await.unwrap();

        if let Some(Value::Set(set)) = value {
            assert_eq!(set.len(), 3);
            assert!(set.contains(&Value::String("a".into())));
            assert!(set.contains(&Value::String("b".into())));
            assert!(set.contains(&Value::Integer(1)));
        } else {
            panic!("Expected Set value");
        }
    }

    #[tokio::test]
    async fn test_parse_push() {
        let mut reader = b">2\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n".as_ref();
        let value = Resp::from3(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(value_push!(
                Value::String("message".into()),
                Value::String("channel".into())
            )),
        );
    }

    #[tokio::test]
    async fn test_parse_nil_resp3() {
        let mut reader = b"_\r\n".as_ref();
        let value = Resp::from3(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Nil));
    }

    #[tokio::test]
    async fn test_round_trip_resp2() {
        // Test values that should round-trip exactly
        // Note: Simple("OK") and Simple("PONG") are parsed as Ok/Pong variants
        let values = vec![
            Value::Error("ERR test".into()),
            Value::Integer(42),
            Value::String("test".into()),
            Value::Nil,
            value_multi!(Value::Integer(1), Value::Integer(2)),
        ];

        for original in values {
            let mut buff = Vec::new();
            Resp::to2(&original, &mut buff).await.unwrap();

            let parsed = Resp::from2(&mut buff.as_ref()).await.unwrap().unwrap();

            assert_eq!(original, parsed, "Round trip failed for {original:?}");
        }

        // Ok and Pong have special parsing behavior
        let mut buff = Vec::new();
        Resp::to2(&Value::Ok, &mut buff).await.unwrap();
        let parsed = Resp::from2(&mut buff.as_ref()).await.unwrap().unwrap();
        assert_eq!(parsed, Value::Ok);

        let mut buff = Vec::new();
        Resp::to2(&Value::Pong, &mut buff).await.unwrap();
        let parsed = Resp::from2(&mut buff.as_ref()).await.unwrap().unwrap();
        assert_eq!(parsed, Value::Pong);
    }

    #[tokio::test]
    async fn test_expired_value() {
        use tokio::time::Duration;

        let expired = Value::Expire((
            Arc::new(Value::String("test".into())),
            Instant::now() - Duration::from_secs(1),
        ));

        assert!(expired.expired());
        assert!(expired.is_none());

        let mut buff = Vec::new();
        Resp::to2(&expired, &mut buff).await.unwrap();
        assert_eq!(buff, b"$-1\r\n"); // Should serialize as Nil
    }

    #[tokio::test]
    async fn test_not_expired_value() {
        use tokio::time::Duration;

        let not_expired = Value::Expire((
            Arc::new(Value::String("test".into())),
            Instant::now() + Duration::from_secs(60),
        ));

        assert!(!not_expired.expired());
        assert!(not_expired.is_some());

        let mut buff = Vec::new();
        Resp::to2(&not_expired, &mut buff).await.unwrap();
        assert_eq!(buff, b"$4\r\ntest\r\n"); // Should serialize inner value
    }

    #[tokio::test]
    async fn test_nested_arrays() {
        let value = value_multi!(
            value_multi!(Value::Integer(1), Value::Integer(2)),
            value_multi!(Value::String("a".into()), Value::String("b".into())),
        );

        let mut buff = Vec::new();
        Resp::to2(&value, &mut buff).await.unwrap();

        let parsed = Resp::from2(&mut buff.as_ref()).await.unwrap().unwrap();

        assert_eq!(value, parsed);
    }

    #[tokio::test]
    async fn test_value_inner() {
        use tokio::time::Duration;

        let inner = Value::String("test".into());
        let expired = Value::Expire((
            Arc::new(inner.clone()),
            Instant::now() + Duration::from_secs(60),
        ));

        assert_eq!(expired.inner(), &inner);
        assert_eq!(inner.inner(), &inner);
    }
}
