use crate::connection::Connection;
use anyhow::Result;
use std::net::TcpListener;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    pub fn bind(addr: &str) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
        })
    }

    pub fn start(&self) -> Result<()> {
        loop {
            let (stream, client) = self.listener.accept()?;
            dbg!(client);

            let mut connection = Connection::new(stream);
            connection.process()?
        }
    }
}
