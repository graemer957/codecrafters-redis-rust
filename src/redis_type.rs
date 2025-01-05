use std::str;

#[derive(Debug, PartialEq, Eq)]
pub enum RedisType<'a> {
    SimpleString(&'a str),
    SimpleError(&'a str),
    BulkString(&'a [u8]),
    Array(Vec<RedisType<'a>>),
}

impl RedisType<'_> {
    pub fn encode(&self) -> Vec<u8> {
        let mut value = Vec::new();

        match self {
            RedisType::SimpleString(string) => {
                value.extend(b"+");
                value.extend(string.as_bytes());
                value.extend(b"\r\n");
            }
            RedisType::SimpleError(error) => {
                value.extend(b"-");
                value.extend(error.as_bytes());
                value.extend(b"\r\n");
            }
            RedisType::BulkString(bulk) => {
                value.extend(b"$");
                value.extend(format!("{}", bulk.len()).as_bytes());
                value.extend(b"\r\n");
                value.extend(*bulk);
                value.extend(b"\r\n");
            }
            RedisType::Array(_vec) => todo!(),
        }

        value
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyValue,
    UnknownType(char),
    UnterminatedSequence,
    InvalidUTF8,
    InvalidUnsigned32BitNumber,
    InvalidLength,
    ExtraBytes,
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownType(x) => write!(f, "Unknown Type `{x}`"),
            Self::EmptyValue => write!(f, "Tried to create type from nothing!"),
            Self::UnterminatedSequence => write!(f, "Missing \\r\\n termination"),
            Self::InvalidUTF8 => write!(f, "Unable to convert [u8] to valid UTF8"),
            Self::InvalidUnsigned32BitNumber => write!(f, "Invalid u32"),
            Self::InvalidLength => write!(f, "Invalid length supplied"),
            Self::ExtraBytes => write!(f, "Extra bytes exist at the end of the parsed type"),
        }
    }
}

impl std::error::Error for Error {}

macro_rules! find_crlf {
    ($value:expr, $on_found:expr) => {
        if let Some(cr) = $value.windows(2).position(|x| x == b"\r\n") {
            $on_found(cr)
        } else {
            Err(Error::UnterminatedSequence)
        }
    };
}

macro_rules! find_length {
    ($value:expr, $cr:expr) => {{
        // TODO: &[u8] -> String -> parse
        let length = &$value[1..$cr];
        let string = String::from_utf8(length.to_vec()).map_err(|_| Error::InvalidUTF8)?;
        let length = string
            .parse::<u32>()
            .map_err(|_| Error::InvalidUnsigned32BitNumber)?;

        Ok((length, &$value[$cr + 2..]))
    }};
}

impl<'a> TryFrom<&'a [u8]> for RedisType<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        fn try_with_remaining(value: &[u8]) -> Result<(RedisType, &[u8]), Error> {
            match value[0] {
                b'+' => find_crlf!(value, |cr| {
                    let result = str::from_utf8(&value[1..cr]).map_err(|_| Error::InvalidUTF8)?;
                    Ok((RedisType::SimpleString(result), &value[cr + 2..]))
                }),
                b'-' => find_crlf!(value, |cr| {
                    let result = str::from_utf8(&value[1..cr]).map_err(|_| Error::InvalidUTF8)?;
                    Ok((RedisType::SimpleError(result), &value[cr + 2..]))
                }),
                b'$' => {
                    let (length, remaining) = find_crlf!(value, |cr| find_length!(value, cr))?;

                    find_crlf!(remaining, |cr| {
                        let value = &remaining[..cr];
                        if value.len() == length as usize {
                            Ok((RedisType::BulkString(value), &remaining[cr + 2..]))
                        } else {
                            Err(Error::InvalidLength)
                        }
                    })
                }
                b'*' => {
                    let (length, remaining) = find_crlf!(value, |cr| find_length!(value, cr))?;
                    let mut elements = Vec::new();
                    let mut remaining = remaining;

                    for _ in 1..=length {
                        let (element, remainder) = try_with_remaining(remaining)?;
                        elements.push(element);
                        remaining = remainder;
                    }

                    Ok((RedisType::Array(elements), remaining))
                }
                _ => Err(Error::UnknownType(char::from(value[0]))),
            }
        }

        if value.is_empty() {
            return Err(Error::EmptyValue);
        }

        let (result, remaining) = try_with_remaining(value)?;
        if remaining.is_empty() {
            Ok(result)
        } else {
            Err(Error::ExtraBytes)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty_should_not_have_a_type() {
        let input: &[u8] = b"";
        assert_eq!(RedisType::try_from(input), Err(Error::EmptyValue))
    }

    #[test]
    fn unknown_type_should_fail() {
        let input: &[u8] = b"z\r\n...\r\n";
        assert_eq!(RedisType::try_from(input), Err(Error::UnknownType('z')));
    }

    #[test]
    fn parse_simple_string() -> Result<(), Error> {
        let input: &[u8] = b"+OK\r\n";
        let result = RedisType::try_from(input)?;

        assert_eq!(result, RedisType::SimpleString("OK"));

        Ok(())
    }

    #[test]
    fn parse_bulk_string() -> Result<(), Error> {
        let input: &[u8] = b"$4\r\nRust\r\n";
        let result = RedisType::try_from(input)?;

        assert_eq!(result, RedisType::BulkString(b"Rust"));

        Ok(())
    }

    #[test]
    fn invalid_u32() {
        let input: &[u8] = b"$h\r\nhello\r\n";
        assert_eq!(
            RedisType::try_from(input),
            Err(Error::InvalidUnsigned32BitNumber)
        );
    }

    #[test]
    fn invalid_length() {
        let input: &[u8] = b"$2\r\nRust\r\n";
        assert_eq!(RedisType::try_from(input), Err(Error::InvalidLength));
    }

    #[test]
    fn parse_array_same_type() -> Result<(), Error> {
        let input: &[u8] = b"*1\r\n$4\r\nPING\r\n";
        let result = RedisType::try_from(input)?;

        assert_eq!(
            result,
            RedisType::Array(vec![RedisType::BulkString(b"PING")])
        );

        Ok(())
    }

    #[test]
    fn parse_array_mixed_type() -> Result<(), Error> {
        let input: &[u8] = b"*2\r\n$4\r\nPING\r\n+OK\r\n";
        let result = RedisType::try_from(input)?;

        assert_eq!(
            result,
            RedisType::Array(vec![
                RedisType::BulkString(b"PING"),
                RedisType::SimpleString("OK")
            ])
        );

        Ok(())
    }

    #[test]
    fn extra_bytes() {
        let input: &[u8] = b"+Sure\r\njunk...";
        assert_eq!(RedisType::try_from(input), Err(Error::ExtraBytes));
    }

    #[test]
    fn invalid_utf8_in_simple_string() {
        // Invalid UTF-8 starting with + and finishing with \r\n
        let input: &[u8] = &[0x2B, 0xF0, 0x28, 0x8C, 0x28, 0x0D, 0x0A];
        assert_eq!(RedisType::try_from(input), Err(Error::InvalidUTF8));
    }

    #[test]
    fn invalid_utf8_in_length() {
        // Invalid UTF-8 starting with $2 and finishing with \r\n
        let input: &[u8] = &[0x24, 0x32, 0xF0, 0x28, 0x8C, 0x28, 0x0D, 0x0A];
        assert_eq!(RedisType::try_from(input), Err(Error::InvalidUTF8));
    }

    #[test]
    fn parse_simple_error() -> Result<(), Error> {
        let input: &[u8] = b"-ERR unknown command 'asdf'\r\n";
        let result = RedisType::try_from(input)?;

        assert_eq!(result, RedisType::SimpleError("ERR unknown command 'asdf'"));

        Ok(())
    }
}
