use crate::{client::Client, threadpool::ThreadPool};
use anyhow::Result;
use std::net::TcpListener;

pub struct Server {
    listener: TcpListener,
}

impl Server {
    #[allow(clippy::missing_errors_doc)]
    pub fn bind(addr: &str) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
        })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn start(&self) -> Result<()> {
        let pool = ThreadPool::new(4);

        loop {
            let (stream, client_addr) = self.listener.accept()?;
            dbg!(client_addr);

            pool.execute(move || {
                let mut client = Client::new(stream);
                // TODO: No support for `Result` in current `ThreadPool` implementation
                if let Err(error) = client.handle() {
                    eprintln!("Client error: {error}");
                }
            });
        }
    }
}
