use crate::{command::Command, resp_data_types::RESPDataType};
use anyhow::Result;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpStream},
};

pub struct Client {
    stream: TcpStream,
    request_buffer: Vec<u8>,
}

impl Client {
    pub const fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            request_buffer: Vec::new(),
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

            let request: RESPDataType = self.request_buffer.as_slice().try_into()?;
            let response = match request.try_into() {
                Ok(Command::Ping) => RESPDataType::SimpleString("PONG"),
                Ok(Command::Echo(message)) => message,
                Err(error) => RESPDataType::SimpleError(error),
            }
            .encode();

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
