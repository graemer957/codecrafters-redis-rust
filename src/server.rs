use crate::connection::Connection;
use crate::threadpool::ThreadPool;
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
            let (stream, client) = self.listener.accept()?;
            dbg!(client);

            let mut connection = Connection::new(stream);
            pool.execute(move || {
                // TODO: No support for `Result` in current `ThreadPool` implementation
                let _ = connection.process();
            });
        }
    }
}
