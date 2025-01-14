use crate::resp_data_types::{BulkString, RESPDataType};

pub enum Command<'a> {
    Ping,
    Echo(BulkString<'a>),
}

impl<'a> TryFrom<RESPDataType<'a>> for Command<'a> {
    type Error = &'a str;

    fn try_from(value: RESPDataType<'a>) -> Result<Self, Self::Error> {
        let RESPDataType::Array(mut array) = value else {
            return Err("ERR request should be an Array");
        };

        let Some(RESPDataType::BulkString(command)) = array.pop_front() else {
            return Err("ERR no command sent!");
        };

        let Some(command) = command.as_string() else {
            return Err("ERR command is not valid UTF8");
        };

        match command.as_bytes() {
            b"PING" => Ok(Self::Ping),

            // ECHO
            // See: https://redis.io/docs/latest/commands/echo/
            b"ECHO" => {
                if let Some(RESPDataType::BulkString(message)) = array.pop_front() {
                    Ok(Command::Echo(message))
                } else {
                    Err("ERR nothing to echo back")
                }
            }

            _ => Err("ERR unknown command"),
        }
    }
}
