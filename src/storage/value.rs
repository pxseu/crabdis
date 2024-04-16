use std::collections::{HashMap, HashSet, VecDeque};
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
    Multi(VecDeque<Value>),
    Expire((Box<Value>, Instant)),
    Map(HashMap<Value, Value>),

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

    pub fn to_resp2<'a, T>(
        &'a self,
        writer: &'a mut T,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    where
        T: AsyncWriteExt + Unpin + Send,
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

                    for value in v {
                        value.to_resp2(writer).await?;
                    }

                    Ok(())
                }
                Self::Map(h) => {
                    let mut values = VecDeque::with_capacity(h.len() * 2);

                    for (k, v) in h {
                        values.push_back(k.clone());
                        values.push_back(v.clone());
                    }

                    Value::Multi(values).to_resp2(writer).await
                }
                Self::Set(s) => {
                    let mut values = VecDeque::with_capacity(s.len());

                    for v in s {
                        values.push_back(v.clone());
                    }

                    Value::Multi(values).to_resp2(writer).await
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

    pub fn to_resp3<'a, T>(
        &'a self,
        writer: &'a mut T,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>
    where
        T: AsyncWriteExt + Unpin + Send,
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

                // rest of the code is the same as to_resp2
                // edit: not really but clients are forgiving
                _ => self.to_resp2(writer).await,
            }
        })
    }

    pub fn from_resp<'a, T>(
        reader: &'a mut BufReader<&mut T>,
    ) -> Pin<Box<dyn Future<Output = Result<Option<Self>>> + Send + 'a>>
    where
        T: AsyncReadExt + Unpin + Send,
    {
        Box::pin(async move {
            let mut line = String::new();

            reader.read_line(&mut line).await?;

            match line.chars().next() {
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
                    let mut values = VecDeque::with_capacity(len);

                    for _ in 0..len {
                        let value = Self::from_resp(reader).await?;

                        if let Some(value) = value {
                            values.push_back(value);
                        } else {
                            return Ok(None);
                        }
                    }

                    Ok(Some(Self::Multi(values)))
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

impl From<VecDeque<Value>> for Value {
    fn from(value: VecDeque<Value>) -> Self {
        Value::Multi(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Integer(value)
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;
    use tokio::net::{TcpListener, TcpStream};

    use super::*;

    async fn create_tcp_stream<'a>(data: &str) -> TcpStream {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut writer = TcpStream::connect(addr).await.unwrap();
        writer.write(data.as_bytes()).await.unwrap();

        let (stream, _) = listener.accept().await.unwrap();

        stream
    }

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

        let value = Value::Multi(VecDeque::from([
            Value::String("Hello, World!".to_string()),
            Value::Integer(42),
            Value::Nil,
        ]));
        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        println!("{:?}", std::str::from_utf8(&buff).unwrap());

        assert_eq!(buff, b"*3\r\n$13\r\nHello, World!\r\n:42\r\n$-1\r\n");

        let value = Value::Multi(VecDeque::from([
            Value::String("key".to_string()),
            Value::String("value".to_string()),
        ]));

        let mut buff = Vec::new();
        value.to_resp2(&mut buff).await.unwrap();

        assert_eq!(buff, b"*2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");
    }

    #[tokio::test]
    async fn test_value_from_resp() {
        let mut stream = create_tcp_stream("$13\r\nHello, World!\r\n").await;
        let (mut read, _) = stream.split();
        let mut reader = BufReader::new(&mut read);

        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::String("Hello, World!".to_string())));

        let mut stream = create_tcp_stream(":42\r\n").await;
        let (mut read, _) = stream.split();
        let mut reader = BufReader::new(&mut read);

        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Integer(42)));

        let mut stream = create_tcp_stream("$-1\r\n").await;
        let (mut read, _) = stream.split();
        let mut reader = BufReader::new(&mut read);

        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(value, Some(Value::Nil));

        let mut stream = create_tcp_stream("*3\r\n$13\r\nHello, World!\r\n:42\r\n$-1\r\n").await;
        let (mut read, _) = stream.split();
        let mut reader = BufReader::new(&mut read);

        let value = Value::from_resp(&mut reader).await.unwrap();
        assert_eq!(
            value,
            Some(Value::Multi(VecDeque::from([
                Value::String("Hello, World!".to_string()),
                Value::Integer(42),
                Value::Nil
            ])))
        );

        let mut stream = create_tcp_stream("*2\r\n$3\r\nkey\r\n$5\r\nvalue\r\n").await;
        let (mut read, _) = stream.split();
        let mut reader = BufReader::new(&mut read);

        let value = Value::from_resp(&mut reader).await.unwrap();
        let mut hashmap = HashMap::new();
        hashmap.insert("key".to_string(), Value::String("value".to_string()));
        assert_eq!(
            value,
            // it wont be a hashmap by default since there is no spec for hashmaps in RESP
            Some(Value::Multi(VecDeque::from([
                Value::String("key".to_string()),
                Value::String("value".to_string())
            ])))
        );
    }
}
