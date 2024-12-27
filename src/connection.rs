use anyhow::Result;
use std::{io::Read, net::TcpStream};

pub struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
        }
    }

    pub fn process(&mut self) -> Result<()> {
        loop {
            let mut buffer = [0; 1024];
            let read = self.stream.read(&mut buffer)?;
            println!("read {read} bytes from stream");

            if read == 0 {
                println!("nothing to read from stream, exiting process loop");
                break;
            }

            self.buffer.extend_from_slice(&buffer[..read]);

            if read < buffer.len() {
                println!("read less than buffer size in, exiting process loop");
                break;
            }
        }

        if let Ok(request) = String::from_utf8(self.buffer.clone()) {
            dbg!(request);
        }

        Ok(())
    }
}
