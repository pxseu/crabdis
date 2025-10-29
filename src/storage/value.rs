use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use tokio::io::{AsyncBufReadExt, AsyncReadExt};
use tokio::time::Instant;

use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Value {
    Ok,   // only for response
    Pong, // only for response
    Nil,
    Simple(Arc<str>),
    Error(Arc<str>),
    Integer(i64),
    String(Arc<str>),
    Multi(Arc<[Value]>),
    Expire((Arc<Value>, Instant)),
    Map(HashMap<Value, Value>),
    Push(Arc<[Value]>), // For RESP3 push messages (pub/sub)

    // not implemented yet
    Set(HashSet<Value>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        use Value::*;
        match (self, other) {
            (Ok, Ok) | (Pong, Pong) | (Nil, Nil) => true,
            (Simple(a), Simple(b)) => a == b,
            (Error(a), Error(b)) => a == b,
            (Integer(a), Integer(b)) => a == b,
            (String(a), String(b)) => a == b,
            (Multi(a), Multi(b)) => a == b,
            (Push(a), Push(b)) => a == b,
            (Map(a), Map(b)) => a == b,
            (Set(a), Set(b)) => a == b,
            // Compare only inner values; ignore Instant to match Hash
            (Expire(_), Expire(_)) => unreachable!("Expire values should not be compared"),
            _ => false,
        }
    }
}

impl Eq for Value {}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Ok => "ok".hash(state),
            Self::Nil => "nil".hash(state),
            Self::Pong => "pong".hash(state),
            Self::Simple(s) => s.hash(state),
            Self::Error(e) => e.hash(state),
            Self::Integer(i) => i.hash(state),
            Self::String(s) => s.hash(state),
            Self::Multi(v) => v.hash(state),
            Self::Expire((v, _)) => v.hash(state),
            Self::Push(v) => v.hash(state),

            Self::Map(_) | Self::Set(_) => unreachable!(),
        }
    }
}

macro_rules! value_error {
    ($($arg:tt)*) => {
        Value::Error(format!($($arg)*).into())
    };
}

pub(crate) use value_error;

impl Value {
    pub fn expired(&self) -> bool {
        match self {
            Self::Expire((_, expires_at)) => Instant::now() > *expires_at,
            _ => false,
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            Self::Nil => false,
            _ if Self::expired(&self) => false,
            _ => true,
        }
    }

    pub fn inner(&self) -> &Self {
        match self {
            Self::Expire((v, _)) => v.inner(),
            _ => self,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn to_resp2<'b, T>(
        &'b self,
        writer: &'b mut T,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'b>>
    where
        T: AsyncWriteExt + Unpin + Send + 'b + ?Sized,
    {
        Box::pin(async move {
            match self {
                Self::Ok => Self::Simple("OK".into()).to_resp2(writer).await,
                Self::Pong => Self::Simple("PONG".into()).to_resp2(writer).await,
                Self::Nil => Ok(writer.write_all(b"$-1\r\n").await?),
                Self::Simple(s) => {
                    writer.write_all(b"+").await?;
                    writer.write_all(s.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                Self::Error(e) => {
                    writer.write_all(b"-").await?;
                    writer.write_all(e.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                Self::Integer(i) => {
                    writer.write_all(b":").await?;
                    writer.write_all(i.to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                Self::String(s) if s.len() == 0 => Ok(writer.write_all(b"$-1\r\n").await?),
                Self::String(s) => {
                    writer.write_all(b"$").await?;
                    writer.write_all(s.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;
                    writer.write_all(s.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }
                // Self::Multi(v) | Self::Push(v) if v.len() == 0 => {
                //     Ok(writer.write_all(b"*0\r\n").await?)
                // }
                Self::Multi(v) | Self::Push(v) => {
                    writer.write_all(b"*").await?;
                    writer.write_all(v.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    for value in v.iter() {
                        value.to_resp2(writer).await?;
                    }

                    Ok(())
                }
                // Self::Map(h) if h.len() == 0 => Ok(writer.write_all(b"*0\r\n").await?),
                Self::Map(h) => {
                    writer.write_all(b"*").await?;
                    // map in non resp3 is serialized as a list of key-value pairs
                    let len = h.len() * 2;
                    writer.write_all(len.to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    for (k, v) in h {
                        k.to_resp2(writer).await?;
                        v.to_resp2(writer).await?;
                    }

                    Ok(())
                }
                // Self::Set(s) if s.len() == 0 => Ok(writer.write_all(b"*0\r\n").await?),
                Self::Set(s) => {
                    writer.write_all(b"*").await?;
                    writer.write_all(s.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    for v in s {
                        v.to_resp2(writer).await?;
                    }

                    Ok(())
                }

                Self::Expire((v, _)) => {
                    // check if the value is expired
                    if Self::expired(&self) {
                        return Self::Nil.to_resp2(writer).await;
                    }

                    v.to_resp2(writer).await
                }
            }
        })
    }

    pub fn to_resp3<'b, T>(
        &'b self,
        writer: &'b mut T,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'b>>
    where
        T: AsyncWriteExt + Unpin + Send + 'b + ?Sized,
    {
        Box::pin(async move {
            match self {
                Self::Nil => Ok(writer.write_all(b"$_\r\n").await?),
                Self::Map(map) => {
                    writer.write_all(b"%").await?;
                    writer.write_all(map.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    for (k, v) in map {
                        k.to_resp3(writer).await?;
                        v.to_resp3(writer).await?;
                    }

                    Ok(())
                }

                Self::Set(set) => {
                    writer.write_all(b"~").await?;
                    writer.write_all(set.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    for v in set {
                        v.to_resp3(writer).await?;
                    }

                    Ok(())
                }

                Self::Error(s) => {
                    writer.write_all(b"!").await?;
                    writer.write_all(s.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;
                    writer.write_all(s.as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    Ok(())
                }

                Self::Push(v) => {
                    writer.write_all(b">").await?;
                    writer.write_all(v.len().to_string().as_bytes()).await?;
                    writer.write_all(b"\r\n").await?;

                    for value in v.iter() {
                        value.to_resp3(writer).await?;
                    }

                    Ok(())
                }

                // rest of the code is the same as to_resp2
                _ => self.to_resp2(writer).await,
            }
        })
    }

    pub fn from_resp<'a, T>(
        reader: &'a mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self>>> + Send + 'a>>
    where
        T: AsyncBufReadExt + Unpin + Send + 'a,
    {
        Box::pin(async move {
            let mut line = String::new();

            reader.read_line(&mut line).await?;

            match line.chars().next() {
                Some('>') => {
                    let len: usize = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;

                    let mut values = Vec::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from_resp(reader).await?;

                        if let Some(value) = value {
                            values.push(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(Self::Push(values.into())))
                }
                Some('$') if line == "$-1\r\n" => Ok(Some(Self::Nil)),

                Some('$') => {
                    let len: usize = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;

                    // +2 for `\r\n
                    let mut value = vec![0; len];
                    reader.read_exact(&mut value).await?;

                    // +2 for `\r\n
                    reader.read_exact(&mut [0; 2]).await?;

                    // SAFETY: we know that the value is a valid utf8 string, or at least it should
                    let value = unsafe { std::str::from_utf8_unchecked(&value) };

                    Ok(Some(Self::String(value.into())))
                }

                Some(':') => {
                    let value: i64 = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;

                    Ok(Some(Self::Integer(value)))
                }

                Some('*') if line == "*0\r\n" => Ok(Some(Self::Multi([].into()))),
                Some('*') => {
                    let len: usize = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;
                    let mut values = Vec::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from_resp(reader).await?;

                        if let Some(value) = value {
                            values.push(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(Self::Multi(values.into())))
                }

                Some('%') => {
                    let len: usize = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;
                    let mut map = HashMap::with_capacity(len);

                    for _ in 0..len {
                        let key = Self::from_resp(reader).await?;
                        let value = Self::from_resp(reader).await?;

                        match (key, value) {
                            (Some(key), Some(value)) => {
                                map.insert(key, value);
                            }
                            _ => return Ok(None),
                        }
                    }

                    Ok(Some(Self::Map(map)))
                }

                Some('~') => {
                    let len: usize = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;
                    let mut set = HashSet::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from_resp(reader).await?;

                        if let Some(value) = value {
                            set.insert(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(Self::Set(set)))
                }

                Some('+') => {
                    let value = line[1..].trim();

                    match value {
                        "OK" => Ok(Some(Self::Ok)),
                        "PONG" => Ok(Some(Self::Pong)),
                        _ => unreachable!("Invalid response"),
                    }
                }

                Some('-') => Ok(Some(Self::Error(line[1..].trim().into()))),

                None => Ok(None),

                _ => Ok(Some(Self::Error("Invalid response".into()))),
            }
        })
    }

    pub fn from_resp3<'a, T>(
        reader: &'a mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self>>> + Send + 'a>>
    where
        T: AsyncBufReadExt + Unpin + Send + 'a,
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
                    let mut buf = vec![];
                    reader.read_until(b'\n', &mut buf).await?;

                    let size = unsafe { str::from_utf8_unchecked(&buf[1..buf.len() - 2]) };
                    let len = size.parse().context("Could not parse integer")?;

                    let mut values = Vec::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from_resp3(reader).await?;

                        if let Some(value) = value {
                            values.push(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(match first_byte {
                        b'>' => Self::Push(values.into()),
                        b'*' => Self::Multi(values.into()),
                        b'~' => {
                            // force no allocation
                            let mut set = HashSet::with_capacity(0);
                            // extend reserves the capacity for us hitting the allocator
                            set.extend(values.into_iter());
                            Self::Set(set)
                        }
                        _ => unreachable!(),
                    }))
                }

                _ => Self::from_resp(reader).await,
            }
        })
    }
}

impl From<Option<Value>> for Value {
    fn from(value: Option<Value>) -> Self {
        value.unwrap_or(Value::Nil)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value.into())
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Value::Multi(value.into())
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_resp2_simple_string() {
        let value = Value::Simple("OK".into());
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"+OK\r\n");
    }

    #[tokio::test]
    async fn test_resp2_error() {
        let value = Value::Error("ERR something went wrong".into());
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"-ERR something went wrong\r\n");
    }

    #[tokio::test]
    async fn test_resp2_integer() {
        let value = Value::Integer(42);
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b":42\r\n");

        let value = Value::Integer(-100);
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b":-100\r\n");

        let value = Value::Integer(0);
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b":0\r\n");
    }

    #[tokio::test]
    async fn test_resp2_bulk_string() {
        let value = Value::String("Hello, World!".into());
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"$13\r\nHello, World!\r\n");

        // Empty string
        let value = Value::String("".into());
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"$-1\r\n");
    }

    #[tokio::test]
    async fn test_resp2_nil() {
        let value = Value::Nil;
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
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
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"*3\r\n$13\r\nHello, World!\r\n:42\r\n$-1\r\n");

        // Empty array
        let value = Value::Multi(Vec::new().into());
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"*0\r\n");
    }

    #[tokio::test]
    async fn test_resp2_map() {
        let mut map = HashMap::new();
        map.insert(Value::String("key1".into()), Value::String("value1".into()));
        map.insert(Value::String("key2".into()), Value::Integer(42));

        let value = Value::Map(map);
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

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
        value.to_resp2(&mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with("*3\r\n"));
    }

    #[tokio::test]
    async fn test_resp2_ok_pong() {
        let value = Value::Ok;
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"+OK\r\n");

        let value = Value::Pong;
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"+PONG\r\n");
    }

    #[tokio::test]
    async fn test_resp3_nil() {
        let value = Value::Nil;
        let mut buff = Vec::new();
        value.to_resp3(&mut buff).await.unwrap();
        assert_eq!(buff, b"$_\r\n");
    }

    #[tokio::test]
    async fn test_resp3_map() {
        let mut map = HashMap::new();
        map.insert(Value::String("key".into()), Value::String("value".into()));

        let value = Value::Map(map);
        let mut buff = Vec::new();
        value.to_resp3(&mut buff).await.unwrap();

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
        value.to_resp3(&mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with("~2\r\n")); // ~ indicates set in RESP3
    }

    #[tokio::test]
    async fn test_resp3_error() {
        let value = Value::Error("ERR test error".into());
        let mut buff = Vec::new();
        value.to_resp3(&mut buff).await.unwrap();
        assert_eq!(buff, b"!14\r\nERR test error\r\n");
    }

    #[tokio::test]
    async fn test_resp3_push() {
        let value = Value::Push(
            Vec::from([
                Value::String("message".into()),
                Value::String("channel".into()),
            ])
            .into(),
        );
        let mut buff = Vec::new();
        value.to_resp3(&mut buff).await.unwrap();

        let resp = String::from_utf8(buff).unwrap();
        assert!(resp.starts_with(">2\r\n")); // > indicates push in RESP3
    }

    #[tokio::test]
    async fn test_parse_simple_string() {
        let buff = b"+OK\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Ok));

        let buff = b"+PONG\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Pong));
    }

    #[tokio::test]
    async fn test_parse_error() {
        let buff = b"-ERR unknown command\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Error("ERR unknown command".into())));
    }

    #[tokio::test]
    async fn test_parse_integer() {
        let buff = b":42\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(42)));

        let buff = b":-100\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(-100)));
    }

    #[tokio::test]
    async fn test_parse_bulk_string() {
        let buff = b"$5\r\nhello\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::String("hello".into())));

        let buff = b"$-1\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Nil));
    }

    #[tokio::test]
    async fn test_parse_array() {
        let buff = b"*3\r\n$5\r\nhello\r\n:42\r\n$-1\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(Value::Multi(
                Vec::from([
                    Value::String("hello".into()),
                    Value::Integer(42),
                    Value::Nil
                ])
                .into(),
            ))
        );

        let buff = b"*0\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Multi(Vec::new().into())));
    }

    #[tokio::test]
    async fn test_parse_map() {
        let buff = b"%2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n$4\r\nkey2\r\n:42\r\n".as_ref();
        let mut reader = Cursor::new(buff);
        let value = Value::from_resp(&mut reader).await.unwrap();

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
        let value = Value::from_resp(&mut reader).await.unwrap();

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
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(Value::Push(
                Vec::from([
                    Value::String("message".into()),
                    Value::String("channel".into()),
                ])
                .into(),
            ))
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
            Value::Multi(vec![Value::Integer(1), Value::Integer(2)].into()),
        ];

        for original in values {
            let mut buff = Vec::new();
            original.to_resp2(&mut buff).await.unwrap();

            let mut reader = Cursor::new(&buff);
            let parsed = Value::from_resp(&mut reader).await.unwrap().unwrap();

            assert_eq!(original, parsed, "Round trip failed for {:?}", original);
        }

        // Ok and Pong have special parsing behavior
        let mut buff = Vec::new();
        Value::Ok.to_resp2(&mut buff).await.unwrap();
        let mut reader = Cursor::new(&buff);
        let parsed = Value::from_resp(&mut reader).await.unwrap().unwrap();
        assert_eq!(parsed, Value::Ok);

        let mut buff = Vec::new();
        Value::Pong.to_resp2(&mut buff).await.unwrap();
        let mut reader = Cursor::new(&buff);
        let parsed = Value::from_resp(&mut reader).await.unwrap().unwrap();
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
        expired.to_resp2(&mut buff).await.unwrap();
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
        not_expired.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"$4\r\ntest\r\n"); // Should serialize inner value
    }

    #[tokio::test]
    async fn test_nested_arrays() {
        let value = Value::Multi(
            vec![
                Value::Multi(vec![Value::Integer(1), Value::Integer(2)].into()),
                Value::Multi(vec![Value::String("a".into()), Value::String("b".into())].into()),
            ]
            .into(),
        );

        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        let mut reader = Cursor::new(&buff);
        let parsed = Value::from_resp(&mut reader).await.unwrap().unwrap();

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
