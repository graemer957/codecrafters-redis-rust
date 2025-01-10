use crate::resp_data_types::{BulkString, RESPDataType, SimpleError};

pub enum Command<'a> {
    Ping,
    Echo(BulkString<'a>),
}

impl<'a> TryFrom<RESPDataType<'a>> for Command<'a> {
    type Error = SimpleError<'a>;

    fn try_from(value: RESPDataType<'a>) -> Result<Self, Self::Error> {
        let RESPDataType::Array(mut array) = value else {
            return Err(SimpleError::new("ERR request should be an Array"));
        };

        let Some(RESPDataType::BulkString(command)) = array.pop_front() else {
            return Err(SimpleError::new("ERR no command sent!"));
        };

        let Some(command) = command.as_string() else {
            return Err(SimpleError::new("ERR command is not valid UTF8"));
        };

        match command.as_bytes() {
            b"PING" => Ok(Self::Ping),

            // ECHO
            // See: https://redis.io/docs/latest/commands/echo/
            b"ECHO" => {
                if let Some(RESPDataType::BulkString(message)) = array.pop_front() {
                    Ok(Command::Echo(message))
                } else {
                    Err(SimpleError::new("ERR nothing to echo back"))
                }
            }

            _ => Err(SimpleError::new("ERR unknown command")),
        }
    }
}
