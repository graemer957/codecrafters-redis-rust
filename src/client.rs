use crate::{
    command::Command,
    resp::{BulkString, NullBulkString, RespType, SimpleError, SimpleString},
    store::Store,
};
use anyhow::Result;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
    str,
    sync::Arc,
};

pub struct Client {
    stream: TcpStream,
    request_buffer: Vec<u8>,
    store: Arc<Store>,
}

impl Client {
    pub const fn new(stream: TcpStream, store: Arc<Store>) -> Self {
        Self {
            stream,
            request_buffer: Vec::new(),
            store,
        }
    }

    pub fn handle(&mut self) -> Result<()> {
        'request: loop {
            loop {
                let mut read_buffer = [0; 1024];
                let read = self.stream.read(&mut read_buffer)?;
                println!("read {read} bytes from stream");

                if read == 0 {
                    if read_buffer == [0; 1024] {
                        println!("no requests left from client, shutting down connection");
                        break 'request;
                    }

                    println!("nothing to read from stream, exiting request loop");
                    break;
                }

                self.request_buffer.extend_from_slice(&read_buffer[..read]);

                if read < read_buffer.len() {
                    println!("read less than buffer size in, exiting request loop");
                    break;
                }
            }

            if let Ok(request) = str::from_utf8(self.request_buffer.as_slice()) {
                dbg!(request);
            }

            let request: RespType = self.request_buffer.as_slice().try_into()?;
            let response = match request.try_into() {
                Ok(command) => match command {
                    Command::Ping => SimpleString::new("PONG").encode(),
                    Command::Echo(message) => message.encode(),
                    Command::Set(key, value) => {
                        if let Some(key) = key.as_string() {
                            // TODO: Are copies for key/value needed?
                            self.store.set(key.to_string(), value.to_vec());
                            SimpleString::new("OK").encode()
                        } else {
                            SimpleError::from("ERR key is not UTF8 string").encode()
                        }
                    }
                    Command::Get(key) => self
                        .store
                        .get(key.as_string().unwrap_or_default())
                        .map_or_else(NullBulkString::encode, |value| {
                            let value: BulkString = value.as_slice().into();
                            value.encode()
                        }),
                },
                Err(error) => Into::<SimpleError>::into(error).encode(),
            };

            if let Ok(response) = str::from_utf8(response.as_slice()) {
                dbg!(response);
            }
            self.stream.write_all(response.as_slice())?;
            self.request_buffer.drain(..);
        }
        self.stream.shutdown(Shutdown::Both)?;

        Ok(())
    }
}
