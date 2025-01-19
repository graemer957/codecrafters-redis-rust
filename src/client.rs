use crate::{
    command::Command,
    resp::{BulkString, RespType, SimpleError, SimpleString},
    store::Store,
};
use anyhow::Result;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
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

            if let Ok(request) = String::from_utf8(self.request_buffer.clone()) {
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
                    Command::Get(key) => {
                        if let Some(value) = self
                            .store
                            .get(key.as_string().unwrap_or_default().to_string())
                        {
                            let response: BulkString = value.as_slice().into();
                            response.encode()
                        } else {
                            SimpleError::from("ERR no value for key").encode()
                        }
                    }
                },
                Err(error) => Into::<SimpleError>::into(error).encode(),
            };

            if let Ok(response) = String::from_utf8(response.clone()) {
                dbg!(response);
            }
            self.stream.write_all(response.as_slice())?;
            self.request_buffer.drain(..);
        }
        self.stream.shutdown(Shutdown::Both)?;

        Ok(())
    }
}
