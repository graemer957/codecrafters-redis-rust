use anyhow::Result;
use redis_starter_rust::Server;

fn main() -> Result<()> {
    let server = Server::bind("127.0.0.1:6379")?;
    server.start()?;

    Ok(())
}
