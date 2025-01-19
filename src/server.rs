use crate::{client::Client, store::Store, threadpool::ThreadPool};
use anyhow::Result;
use std::{net::TcpListener, sync::Arc};

pub struct Server {
    listener: TcpListener,
    store: Arc<Store>,
}

impl Server {
    #[allow(clippy::missing_errors_doc)]
    pub fn bind(addr: &str) -> Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
            store: Arc::new(Store::new()),
        })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn start(&self) -> Result<()> {
        let pool = ThreadPool::new(4);

        loop {
            let (stream, client_addr) = self.listener.accept()?;
            dbg!(client_addr);

            let store = Arc::clone(&self.store);
            pool.execute(move || {
                let mut client = Client::new(stream, store);
                // TODO: No support for `Result` in current `ThreadPool` implementation
                if let Err(error) = client.handle() {
                    eprintln!("Client error: {error}");
                }
            });
        }
    }
}
