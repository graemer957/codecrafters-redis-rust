use crate::resp::{BulkString, RespType};

pub enum Command<'a> {
    Ping,
    Echo(BulkString<'a>),
    Set(BulkString<'a>, BulkString<'a>),
    Get(BulkString<'a>),
}

impl<'a> TryFrom<RespType<'a>> for Command<'a> {
    type Error = &'a str;

    fn try_from(value: RespType<'a>) -> Result<Self, Self::Error> {
        let RespType::Array(mut array) = value else {
            return Err("ERR request should be an Array");
        };

        let Some(RespType::BulkString(command)) = array.pop_front() else {
            return Err("ERR no command sent!");
        };

        let Some(command) = command.as_string() else {
            return Err("ERR command is not valid UTF8");
        };

        match command.to_uppercase().as_bytes() {
            b"PING" => Ok(Self::Ping),

            // ECHO
            // See: https://redis.io/docs/latest/commands/echo/
            b"ECHO" => {
                if let Some(RespType::BulkString(message)) = array.pop_front() {
                    Ok(Command::Echo(message))
                } else {
                    Err("ERR nothing to echo back")
                }
            }

            // SET
            // See: https://redis.io/docs/latest/commands/set/
            b"SET" => {
                let Some(RespType::BulkString(key)) = array.pop_front() else {
                    return Err("ERR missing or incorrectly formatted key");
                };

                let Some(RespType::BulkString(value)) = array.pop_front() else {
                    return Err("ERR missing or incorrectly formatted value");
                };

                Ok(Command::Set(key, value))
            }

            // GET
            // See: https://redis.io/docs/latest/commands/get/
            b"GET" => {
                let Some(RespType::BulkString(key)) = array.pop_front() else {
                    return Err("ERR missing or incorrectly formatted key");
                };

                Ok(Command::Get(key))
            }

            _ => Err("ERR unknown command"),
        }
    }
}
