use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::io::Write;
use std::pin::Pin;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::time::Instant;

use crate::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Ok,            // only for response
    Nil,           // only for response
    Error(String), // only for response
    Pong,          // only for response
    Integer(i64),
    String(String),
    Multi(VecDeque<Value>),
    Expire((Box<Value>, Instant)),

    // i promise i will implement this
    #[allow(dead_code)]
    Hashmap(HashMap<String, Value>),
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
                Self::Ok => Ok(writer.write_all(b"+OK\r\n").await?),
                Self::Pong => Ok(writer.write_all(b"+PONG\r\n").await?),
                Self::Nil => Ok(writer.write_all(b"$-1\r\n").await?),

                Self::Error(e) => {
                    let mut buff = Vec::with_capacity(e.len() + 3);
                    write!(&mut buff, "-{e}\r\n").context("Could not write to buffer")?;

                    writer.write_all(&buff).await?;

                    Ok(())
                }
                Self::Integer(i) => {
                    let string = i.to_string();

                    let mut buff = Vec::with_capacity(string.len() + 3);
                    write!(&mut buff, ":{i}\r\n").context("Could not write to buffer")?;

                    writer.write_all(&buff).await?;

                    Ok(())
                }
                Self::String(s) => {
                    let len = s.len();

                    let mut buff = Vec::with_capacity(len.to_string().len() + len + 5);
                    write!(&mut buff, "${len}\r\n{s}\r\n").context("Could not write to buffer")?;

                    writer.write_all(&buff).await?;

                    Ok(())
                }
                Self::Multi(v) => {
                    let len = v.len();

                    let mut buff = Vec::with_capacity(len.to_string().len() + 3);
                    write!(&mut buff, "*{len}\r\n").context("Could not write to buffer")?;

                    writer.write_all(&buff).await?;

                    for value in v {
                        value.to_resp2(writer).await?;
                    }

                    Ok(())
                }
                Self::Hashmap(h) => {
                    let mut values = VecDeque::with_capacity(h.len() * 2);

                    for (k, v) in h {
                        values.push_back(Value::String(k.clone()));
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
                Self::Hashmap(map) => {
                    let len = map.len();

                    let mut buff = Vec::with_capacity(len.to_string().len() + 3);
                    write!(&mut buff, "%{len}\r\n").context("Could not write to buffer")?;

                    writer.write_all(&buff).await?;

                    for (k, v) in map {
                        writer.write_all(format!("+{k}\r\n").as_bytes()).await?;
                        v.to_resp3(writer).await?;
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
