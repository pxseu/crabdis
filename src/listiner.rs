use std::net::{IpAddr, SocketAddr};

use tokio::net::{TcpListener as TokioTcpListener, TcpStream};

pub struct TcpListener {
    listener: TokioTcpListener,
}

impl TcpListener {
    pub async fn bind(ip: Option<IpAddr>, port: Option<u16>) -> std::io::Result<Self> {
        Ok(Self {
            listener: TokioTcpListener::bind(SocketAddr::new(
                ip.unwrap_or(IpAddr::from([127, 0, 0, 1])),
                port.unwrap_or(6379),
            ))
            .await?,
        })
    }

    pub async fn accept(&self) -> std::io::Result<(TcpStream, std::net::SocketAddr)> {
        let (stream, addr) = self.listener.accept().await?;

        Ok((stream, addr))
    }
}
