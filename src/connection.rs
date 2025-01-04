use crate::redis_type::RedisType;
use anyhow::Result;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
};

pub struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl Connection {
    pub const fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            buffer: Vec::new(),
        }
    }

    pub fn process(&mut self) -> Result<()> {
        'command: loop {
            loop {
                let mut buffer = [0; 1024];
                let read = self.stream.read(&mut buffer)?;
                println!("read {read} bytes from stream");

                if read == 0 {
                    if buffer == [0; 1024] {
                        println!("no commands left from client, shutting down connection");
                        break 'command;
                    }

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

            let request = RedisType::try_from(self.buffer.as_slice())?;
            let response = match request {
                RedisType::Array(array) if array.contains(&RedisType::BulkString(b"PING")) => {
                    RedisType::SimpleString("PONG")
                }
                _ => RedisType::SimpleError("ERR unknown command"),
            }
            .encode();
            if let Ok(response) = String::from_utf8(response.clone()) {
                dbg!(response);
            }
            self.stream.write_all(response.as_slice())?;
            self.buffer.drain(0..);
        }
        self.stream.shutdown(Shutdown::Both)?;

        Ok(())
    }
}
