use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::time::Instant;

use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Ok,   // only for response
    Pong, // only for response
    Nil,
    Simple(String),
    Error(String),
    Integer(i64),
    String(String),
    Multi(Arc<Box<[Value]>>),
    Expire((Box<Value>, Instant)),
    Map(HashMap<Value, Value>),
    Push(Arc<Box<[Value]>>), // For RESP3 push messages (pub/sub)

    // not implemented yet
    Set(HashSet<Value>),
}

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
            // Self::Hashmap(v) => {
            //     use std::collections::hash_map::DefaultHasher;
            //     use std::hash::Hasher;

            //     let mut total_hash = 0;

            //     for (k, v) in v.iter() {
            //         let mut hasher = DefaultHasher::new();

            //         k.hash(&mut hasher);
            //         v.hash(&mut hasher);

            //         let pair_hash = hasher.finish();

            //         // this might be insecure however it does not matter for now
            //         total_hash ^= pair_hash; // XOR the hashes together
            //     }

            //     total_hash.hash(state);
            // }
        }
    }
}

macro_rules! value_error {
    ($($arg:tt)*) => {
        Value::Error(format!($($arg)*))
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
            Self::Expire((v, _)) if !self.expired() => v.inner(),
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
        T: AsyncWriteExt + Unpin + Send + 'b,
    {
        Box::pin(async move {
            match self {
                Self::Ok => Self::to_resp2(&Self::Simple("OK".to_string()), writer).await,
                Self::Pong => Self::to_resp2(&Self::Simple("PONG".to_string()), writer).await,

                Self::Nil => Ok(writer.write_all(b"$-1\r\n").await?),
                Self::Simple(s) => Ok(writer.write_all(format!("+{s}\r\n").as_bytes()).await?),
                Self::Error(e) => Ok(writer.write_all(format!("-{e}\r\n").as_bytes()).await?),
                Self::Integer(i) => Ok(writer.write_all(format!(":{i}\r\n").as_bytes()).await?),
                Self::String(s) => {
                    let len = s.len();

                    writer
                        .write_all(format!("${len}\r\n{s}\r\n").as_bytes())
                        .await?;

                    Ok(())
                }
                Self::Multi(v) => {
                    let len = v.len();

                    writer.write_all(format!("*{len}\r\n").as_bytes()).await?;

                    for value in v.iter() {
                        value.to_resp2(writer).await?;
                    }

                    Ok(())
                }
                Self::Push(v) => Value::Multi(v.clone()).to_resp2(writer).await,
                Self::Map(h) => {
                    let mut values = Vec::with_capacity(h.len() * 2);

                    for (k, v) in h {
                        values.push(k.clone());
                        values.push(v.clone());
                    }

                    Value::Multi(Arc::new(values.into_boxed_slice()))
                        .to_resp2(writer)
                        .await
                }
                Self::Set(s) => {
                    let mut values = Vec::with_capacity(s.len());

                    for v in s {
                        values.push(v.clone());
                    }

                    Value::Multi(Arc::new(values.into_boxed_slice()))
                        .to_resp2(writer)
                        .await
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
        T: AsyncWriteExt + Unpin + Send + 'b,
    {
        Box::pin(async move {
            match self {
                Self::Nil => Ok(writer.write_all(b"$_\r\n").await?),
                Self::Map(map) => {
                    let len = map.len();

                    writer.write_all(format!("%{len}\r\n").as_bytes()).await?;

                    for (k, v) in map {
                        k.to_resp3(writer).await?;
                        v.to_resp3(writer).await?;
                    }

                    Ok(())
                }

                Self::Set(set) => {
                    let len = set.len();

                    writer.write_all(format!("~{len}\r\n").as_bytes()).await?;

                    for v in set {
                        v.to_resp3(writer).await?;
                    }

                    Ok(())
                }

                Self::Error(s) => {
                    let len = s.len();

                    writer
                        .write_all(format!("!{len}\r\n{s}\r\n").as_bytes())
                        .await?;

                    Ok(())
                }

                Self::Push(v) => {
                    let len = v.len();

                    writer.write_all(format!(">{len}\r\n").as_bytes()).await?;

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
        reader: &'a mut BufReader<&mut T>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self>>> + Send + 'a>>
    where
        T: AsyncReadExt + Unpin + Send + 'a,
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

                    let values = values.into_boxed_slice();

                    let values = Arc::new(values);

                    Ok(Some(Self::Push(values)))
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
                    let value = unsafe { String::from_utf8_unchecked(value) };

                    Ok(Some(Self::String(value)))
                }

                Some(':') => {
                    let value: i64 = line[1..]
                        .trim()
                        .parse()
                        .context("Could not parse integer")?;

                    Ok(Some(Self::Integer(value)))
                }

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

                    let values = values.into_boxed_slice();

                    Ok(Some(Self::Multi(Arc::new(values))))
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

                Some('-') => Ok(Some(Self::Error(line[1..].trim().to_string()))),

                None => Ok(None),

                _ => Ok(Some(Self::Error("Invalid response".to_string()))),
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
        Value::String(value)
    }
}

impl From<Vec<Value>> for Value {
    fn from(value: Vec<Value>) -> Self {
        Value::Multi(Arc::new(value.into_boxed_slice()))
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

    #[tokio::test]
    async fn test_value_to_resp() {
        let value = Value::String("Hello, World!".to_string());
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        assert_eq!(buff, b"$13\r\nHello, World!\r\n");

        let value = Value::Integer(42);
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        assert_eq!(buff, b":42\r\n");

        let value = Value::Nil;
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        assert_eq!(buff, b"$-1\r\n");

        let value = Value::Multi(Arc::new(
            Vec::from([
                Value::String("Hello, World!".to_string()),
                Value::Integer(42),
                Value::Nil,
            ])
            .into_boxed_slice(),
        ));
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        println!("{:?}", std::str::from_utf8(&buff).unwrap());

        assert_eq!(buff, b"*3\r\n$13\r\nHello, World!\r\n:42\r\n$-1\r\n");

        let value = Value::Multi(Arc::new(
            Vec::from([
                Value::String("key".to_string()),
                Value::String("value".to_string()),
            ])
            .into_boxed_slice(),
        ));
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"*2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");

        let value = Value::Nil;
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();
        assert_eq!(buff, b"$-1\r\n");
    }

    #[tokio::test]
    async fn test_value_from_resp() {
        let mut buff = b"*3\r\n$13\r\nHello, World!\r\n:42\r\n$-1\r\n".as_ref();
        let mut reader = BufReader::new(&mut buff);
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(Value::Multi(Arc::new(
                Vec::from([
                    Value::String("Hello, World!".to_string()),
                    Value::Integer(42),
                    Value::Nil,
                ])
                .into_boxed_slice()
            )))
        );
        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(42)));

        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Nil));
    }
}
