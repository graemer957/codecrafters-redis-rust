use crate::resp::{BulkString, RespType};
use std::time::Duration;

pub enum Command<'a> {
    Ping,
    Echo(BulkString<'a>),
    Set(BulkString<'a>, BulkString<'a>, Option<Duration>),
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

        match command {
            x if x.eq_ignore_ascii_case("ping") => Ok(Self::Ping),

            // ECHO
            // See: https://redis.io/docs/latest/commands/echo/
            x if x.eq_ignore_ascii_case("echo") => {
                if let Some(RespType::BulkString(message)) = array.pop_front() {
                    Ok(Command::Echo(message))
                } else {
                    Err("ERR nothing to echo back")
                }
            }

            // SET
            // See: https://redis.io/docs/latest/commands/set/
            x if x.eq_ignore_ascii_case("set") => {
                let Some(RespType::BulkString(key)) = array.pop_front() else {
                    return Err("ERR missing or incorrectly formatted key");
                };

                let Some(RespType::BulkString(value)) = array.pop_front() else {
                    return Err("ERR missing or incorrectly formatted value");
                };

                let mut ttl = None;
                while let Some(option) = array.pop_front() {
                    let RespType::BulkString(string) = option else {
                        // TODO: It may be possible to mix and match, but not seeing this in
                        // tests from CodeCrafters thus far
                        return Err("ERR expecting BulkString for option");
                    };

                    match string.as_string() {
                        Some(option) if option.eq_ignore_ascii_case("px") => {
                            if let Some(RespType::BulkString(number)) = array.pop_front() {
                                ttl = number
                                    .as_u64()
                                    .map(Duration::from_millis)
                                    .ok_or("ERR PX no provided in milliseconds")?
                                    .into();
                            } else {
                                return Err("ERR PX duration missing");
                            }
                        }
                        option => eprintln!("skipping over unknown option {option:?}"),
                    }
                }

                Ok(Command::Set(key, value, ttl))
            }

            // GET
            // See: https://redis.io/docs/latest/commands/get/
            x if x.eq_ignore_ascii_case("get") => {
                let Some(RespType::BulkString(key)) = array.pop_front() else {
                    return Err("ERR missing or incorrectly formatted key");
                };

                Ok(Command::Get(key))
            }

            _ => Err("ERR unknown command"),
        }
    }
}
