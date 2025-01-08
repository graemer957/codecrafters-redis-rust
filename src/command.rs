use crate::resp_data_types::{RESPDataType, SimpleError};

pub enum Command<'a> {
    Ping,
    Echo(RESPDataType<'a>),
}

impl<'a> TryFrom<RESPDataType<'a>> for Command<'a> {
    type Error = SimpleError<'a>;

    fn try_from(value: RESPDataType<'a>) -> Result<Self, Self::Error> {
        match value {
            RESPDataType::Array(array)
                if array.starts_with(&[RESPDataType::BulkString(b"PING")]) =>
            {
                Ok(Self::Ping)
            }

            // ECHO
            // See: https://redis.io/docs/latest/commands/echo/
            RESPDataType::Array(array)
                if array.starts_with(&[RESPDataType::BulkString(b"ECHO")]) =>
            {
                if let Some(RESPDataType::BulkString(element)) = array.get(1) {
                    Ok(Command::Echo(RESPDataType::BulkString(element)))
                } else {
                    Err(SimpleError::new("ERR nothing to echo back"))
                }
            }
            _ => Err(SimpleError::new("ERR unknown command")),
        }
    }
}
