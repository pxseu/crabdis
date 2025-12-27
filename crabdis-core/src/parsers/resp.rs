use std::collections::{HashMap, HashSet};
use std::pin::Pin;

use crate::prelude::*;
use crate::value::Value;

/// # Errors
///
/// Returns the original `Ok(true)` value if the reader is not empty, or `Ok(false)` if the reader is empty.
/// Returns an [`Error::Io`] if the reader fails to fill the buffer.
pub async fn can_read<R>(reader: &mut R) -> Result<bool>
where
    R: AsyncBufRead + Unpin,
{
    Ok(!reader.fill_buf().await?.is_empty())
}

/// # Errors
///
/// Returns the original `Ok(Some(Value))` value if successful, or `Ok(None)` if the reader is empty.
/// Returns an [`Error::Io`] if the reader fails to fill the buffer.
pub async fn try_parse<'a, R>(
    reader: &'a mut R,
    version: u8,
) -> Result<Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send + 'a>>>
where
    R: AsyncBufRead + Unpin + Send + 'a,
{
    if !can_read(reader).await? {
        let fut: Pin<Box<dyn Future<Output = Result<Option<Value>>> + Send>> =
            Box::pin(async move { Ok(None) });
        return Ok(fut);
    }

    Ok(match version {
        2 => Resp::from2(reader),
        3 => Resp::from3(reader),
        _ => unreachable!("Invalid protocol version"),
    })
}

pub struct Resp;

impl Resp {
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
                    writer.write_u8(b'+').await?;
                    writer.write_all(s.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                Value::Error(e) => {
                    writer.write_u8(b'-').await?;
                    writer.write_all(e.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                Value::Integer(i) => {
                    writer.write_u8(b':').await?;
                    crate::parsers::int::serialize(writer, *i).await?;

                    Ok(())
                }
                Value::String(s) if s.is_empty() => Ok(writer.write_all(b"$-1\r\n").await?),
                Value::String(s) => {
                    writer.write_u8(b'$').await?;
                    crate::parsers::int::serialize(writer, s.len() as i64).await?;
                    writer.write_all(s.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                Value::Multi(v) | Value::Push(v) => {
                    writer.write_u8(b'*').await?;
                    crate::parsers::int::serialize(writer, v.len() as i64).await?;

                    for value in v.iter() {
                        Self::to2(value, writer).await?;
                    }

                    Ok(())
                }
                Value::Map(h) => {
                    writer.write_u8(b'*').await?;
                    // map in non resp3 is serialized as a list of key-value pairs
                    let len = h.len() * 2;
                    crate::parsers::int::serialize(writer, len as i64).await?;

                    for (k, v) in h {
                        Self::to2(k, writer).await?;
                        Self::to2(v, writer).await?;
                    }

                    Ok(())
                }
                Value::Set(s) => {
                    writer.write_u8(b'*').await?;
                    crate::parsers::int::serialize(writer, s.len() as i64).await?;

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
                Value::Nil => Ok(writer.write_all(b"$_\r\n").await?),
                Value::Map(map) => {
                    writer.write_u8(b'%').await?;
                    crate::parsers::int::serialize(writer, map.len() as i64).await?;

                    for (k, v) in map {
                        Self::to3(k, writer).await?;
                        Self::to3(v, writer).await?;
                    }

                    Ok(())
                }

                Value::Set(set) => {
                    writer.write_u8(b'~').await?;
                    crate::parsers::int::serialize(writer, set.len() as i64).await?;

                    for v in set {
                        Self::to3(v, writer).await?;
                    }

                    Ok(())
                }

                Value::Error(s) => {
                    writer.write_u8(b'!').await?;
                    crate::parsers::int::serialize(writer, s.len() as i64).await?;
                    writer.write_all(s.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }

                Value::Push(v) => {
                    writer.write_u8(b'>').await?;
                    crate::parsers::int::serialize(writer, v.len() as i64).await?;

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
        Box::pin(async move {
            let first_byte = match reader.read_u8().await {
                Ok(byte) => byte,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(e.into()),
            };

            match first_byte {
                b'$' => {
                    let len = crate::parsers::int::deserialize(reader).await?;

                    if len == -1 {
                        return Ok(Some(Value::Nil));
                    }

                    if len == 0 {
                        return Ok(Some(Value::String("".into())));
                    }

                    let mut value = vec![0; len.unsigned_abs() as usize];
                    reader.read_exact(&mut value).await?;

                    // SAFETY: we know that the value is a valid utf8 string, or at least it should
                    let value = unsafe { std::str::from_utf8_unchecked(&value) };

                    // +2 for `\r\n
                    reader.read_exact(&mut [0; 2]).await?;

                    Ok(Some(Value::String(value.into())))
                }

                b':' => {
                    let value = crate::parsers::int::deserialize(reader).await?;

                    Ok(Some(Value::Integer(value)))
                }

                b'*' => {
                    let len = crate::parsers::int::deserialize(reader).await?;

                    if len == 0 {
                        return Ok(Some(Value::Multi([].into())));
                    }

                    let mut values = Vec::with_capacity(len.unsigned_abs() as usize);

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

                b'%' => {
                    let len = crate::parsers::int::deserialize(reader).await?;

                    if len == 0 {
                        return Ok(Some(Value::Map(HashMap::with_capacity(0))));
                    }

                    let mut map = HashMap::with_capacity(len.unsigned_abs() as usize);

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

                b'+' => {
                    let value = crate::parsers::simple::deserialize(reader).await?;

                    match value.as_ref() {
                        "OK" => Ok(Some(Value::Ok)),
                        "PONG" => Ok(Some(Value::Pong)),
                        v => Ok(Some(Value::Simple(v.into()))),
                    }
                }

                b'-' => {
                    let value = crate::parsers::simple::deserialize(reader).await?;

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
        T: AsyncBufRead + Unpin + Send + 'a,
    {
        Box::pin(async move {
            let first_byte = match reader.read_u8().await {
                Ok(byte) => byte,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(e.into()),
            };

            match first_byte {
                // apart from bulk array i dont think any of these can be sent to a server
                b'>' | b'*' | b'~' => {
                    let len = crate::parsers::int::deserialize(reader).await?;

                    let mut values = Vec::with_capacity(len.unsigned_abs() as usize);

                    for _ in 0..len {
                        let value = Self::from3(reader).await?;

                        if let Some(value) = value {
                            values.push(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(match first_byte {
                        b'>' => Value::Push(values.into()),
                        b'*' => Value::Multi(values.into()),
                        b'~' => {
                            // this is basically 1:1 copy of the code that is using .extend
                            let mut set = HashSet::with_capacity(len.unsigned_abs() as usize);

                            // force into_iter to free the values vector after the loop
                            for v in values {
                                set.insert(v);
                            }

                            Value::Set(set)
                        }
                        _ => unreachable!(),
                    }))
                }

                _ => Self::from2(reader).await,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use super::*;
    use crate::{value_multi, value_push};

    use tokio::time::Instant;

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
        assert_eq!(buff, b"$_\r\n");
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
        let buff = b"+OK\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Ok));

        let buff = b"+PONG\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Pong));
    }

    #[tokio::test]
    async fn test_parse_error() {
        let buff = b"-ERR unknown command\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Error("ERR unknown command".into())));
    }

    #[tokio::test]
    async fn test_parse_integer() {
        let buff = b":42\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(42)));

        let buff = b":-100\r\n".as_ref();
        let mut reader = Cursor::new(buff);
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
        let buff = b"*3\r\n$5\r\nhello\r\n:42\r\n$-1\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(value_multi![
                Value::String("hello".into()),
                Value::Integer(42),
                Value::Nil
            ]),
        );

        let buff = b"*0\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(value, Some(value_multi![]));
    }

    #[tokio::test]
    async fn test_parse_map() {
        let buff = b"%2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n$4\r\nkey2\r\n:42\r\n".as_ref();
        let mut reader = Cursor::new(buff);
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
        let buff = b"~3\r\n$1\r\na\r\n$1\r\nb\r\n:1\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();

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
        let buff = b">2\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Resp::from2(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(value_push!(
                Value::String("message".into()),
                Value::String("channel".into())
            )),
        );
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

            let mut reader = Cursor::new(&buff);
            let parsed = Resp::from2(&mut reader).await.unwrap().unwrap();

            assert_eq!(original, parsed, "Round trip failed for {original:?}");
        }

        // Ok and Pong have special parsing behavior
        let mut buff = Vec::new();
        Resp::to2(&Value::Ok, &mut buff).await.unwrap();
        let mut reader = Cursor::new(&buff);
        let parsed = Resp::from2(&mut reader).await.unwrap().unwrap();
        assert_eq!(parsed, Value::Ok);

        let mut buff = Vec::new();
        Resp::to2(&Value::Pong, &mut buff).await.unwrap();
        let mut reader = Cursor::new(&buff);
        let parsed = Resp::from2(&mut reader).await.unwrap().unwrap();
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

        let mut reader = Cursor::new(&buff);
        let parsed = Resp::from2(&mut reader).await.unwrap().unwrap();

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
