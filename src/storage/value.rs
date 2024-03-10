use std::collections::HashMap;

use tokio::io::AsyncReadExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::Duration;

use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum Value {
    Nil, // only for response
    Integer(i64),
    String(String),
    Multi(Vec<Value>),
    Hashmap(HashMap<String, Value>),
    Expire((Box<Value>, Duration)),
    Error(String),
}

impl Value {
    pub fn to_resp(&self) -> String {
        match self {
            Self::Nil => "$-1\r\n".to_string(),
            Self::Integer(i) => format!(":{i}\r\n"),
            Self::String(s) => format!("${}\r\n+{s}\r\n", s.len()),
            Self::Multi(v) => {
                let mut resp = format!("*{}\r\n", v.len());

                for value in v {
                    resp.push_str(&value.to_resp());
                }

                resp
            }
            Self::Hashmap(h) => {
                let mut resp = format!("*{}\r\n", h.len() * 2);
                for (k, v) in h {
                    resp.push_str(&Value::String(k.clone()).to_resp());
                    resp.push_str(&v.to_resp());
                }
                resp
            }
            Self::Expire((v, _)) => v.to_resp(),
            Self::Error(e) => format!("-{e}\r\n"),
        }
    }

    pub async fn from_resp(stream: TcpStream) -> Result<Self> {
        let reader = BufReader::new(stream);

        let mut line = String::new();

        reader.read_line(&mut line).await;

        match line.chars().next() {
            Some('$') if line == "$-1\r\n" => Ok(Self::Nil),

            Some('$') => {
                let len: usize = line[1..].trim().parse().map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "Could not parse intiger")
                })?;

                let mut value = vec![0; len];
                reader.read_exact(&mut value).await?;

                Ok(Self::String(String::from_utf8(value)?))
            }
            Some(':') => {
                let value: i64 = line[1..].trim().parse()?;

                Ok(Self::Integer(value))
            }
            Some('*') => {
                let len: usize = line[1..].trim().parse()?;
                let mut values = Vec::with_capacity(len);

                for _ in 0..len {
                    values.push(Value::from_resp(stream).await?);
                }

                Ok(Self::Multi(values))
            }
            Some('+') => Ok(Self::String(line[1..].trim().to_string())),
            _ => Ok(Self::Error("Invalid response".to_string())),
        }
    }
}
