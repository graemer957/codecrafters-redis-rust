use std::{collections::VecDeque, str};

#[derive(Debug, PartialEq, Eq)]
pub struct SimpleError<'a> {
    inner: &'a str,
}

impl SimpleError<'_> {
    pub fn encode(&self) -> Vec<u8> {
        // 1 for type
        // 2 for terminator
        let mut value = Vec::with_capacity(1 + self.inner.len() + 2);
        value.push(b'-');
        value.extend_from_slice(self.inner.as_bytes());
        value.extend_from_slice(b"\r\n");

        value
    }
}

impl<'a> From<&'a str> for SimpleError<'a> {
    fn from(inner: &'a str) -> Self {
        Self { inner }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SimpleString<'a> {
    inner: &'a str,
}

impl SimpleString<'_> {
    pub const fn new(inner: &str) -> SimpleString<'_> {
        SimpleString { inner }
    }

    pub fn encode(&self) -> Vec<u8> {
        // 1 for type
        // 2 for terminator
        let mut value = Vec::with_capacity(1 + self.inner.len() + 2);
        value.push(b'+');
        value.extend_from_slice(self.inner.as_bytes());
        value.extend_from_slice(b"\r\n");

        value
    }
}

impl<'a> From<&'a str> for SimpleString<'a> {
    fn from(inner: &'a str) -> Self {
        Self { inner }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BulkString<'a> {
    inner: &'a [u8],
}

impl BulkString<'_> {
    pub fn encode(&self) -> Vec<u8> {
        // TODO: Idiomatic way to convert usize to &[u8]?
        let mut value = Vec::new();
        value.extend(b"$");
        value.extend(format!("{}", self.inner.len()).as_bytes());
        value.extend(b"\r\n");
        value.extend(self.inner);
        value.extend(b"\r\n");
        value
    }

    pub fn as_string(&self) -> Option<&str> {
        str::from_utf8(self.inner).ok()
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.inner.to_vec()
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.as_string()
            .and_then(|value| str::parse::<u64>(value).ok())
    }
}

impl<'a> From<&'a [u8]> for BulkString<'a> {
    fn from(inner: &'a [u8]) -> Self {
        Self { inner }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NullBulkString;

impl NullBulkString {
    pub fn encode() -> Vec<u8> {
        b"$-1\r\n".to_vec()
    }
}

// This is needed to support the heterogeneous arrays used in RESP
// See https://redis.io/docs/latest/develop/reference/protocol-spec/#arrays
#[derive(Debug, PartialEq, Eq)]
pub enum RespType<'a> {
    SimpleString(SimpleString<'a>),
    SimpleError(SimpleError<'a>),
    BulkString(BulkString<'a>),
    Array(VecDeque<RespType<'a>>),
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
    ($value:expr_2021, $on_found:expr_2021) => {
        if let Some(cr) = $value.windows(2).position(|x| x == b"\r\n") {
            $on_found(cr)
        } else {
            Err(Error::UnterminatedSequence)
        }
    };
}

macro_rules! find_length {
    ($value:expr_2021, $cr:expr_2021) => {{
        // TODO: &[u8] -> &str -> parse
        let length = str::from_utf8(&$value[1..$cr])
            .map_err(|_| Error::InvalidUTF8)?
            .parse::<u32>()
            .map_err(|_| Error::InvalidUnsigned32BitNumber)?;

        Ok((length, &$value[$cr + 2..]))
    }};
}

impl<'a> TryFrom<&'a [u8]> for RespType<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        fn try_with_remaining(value: &[u8]) -> Result<(RespType, &[u8]), Error> {
            match value[0] {
                b'+' => find_crlf!(value, |cr| {
                    let result = str::from_utf8(&value[1..cr]).map_err(|_| Error::InvalidUTF8)?;
                    Ok((RespType::SimpleString(result.into()), &value[cr + 2..]))
                }),
                b'-' => find_crlf!(value, |cr| {
                    let result = str::from_utf8(&value[1..cr]).map_err(|_| Error::InvalidUTF8)?;
                    Ok((RespType::SimpleError(result.into()), &value[cr + 2..]))
                }),
                b'$' => {
                    let (length, remaining) = find_crlf!(value, |cr| find_length!(value, cr))?;

                    find_crlf!(remaining, |cr| {
                        let value = &remaining[..cr];
                        if value.len() == length as usize {
                            Ok((RespType::BulkString(value.into()), &remaining[cr + 2..]))
                        } else {
                            Err(Error::InvalidLength)
                        }
                    })
                }
                b'*' => {
                    let (length, remaining) = find_crlf!(value, |cr| find_length!(value, cr))?;
                    let mut elements = VecDeque::new();
                    let mut remaining = remaining;

                    for _ in 1..=length {
                        let (element, remainder) = try_with_remaining(remaining)?;
                        elements.push_back(element);
                        remaining = remainder;
                    }

                    Ok((RespType::Array(elements), remaining))
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
        assert_eq!(RespType::try_from(input), Err(Error::EmptyValue))
    }

    #[test]
    fn unknown_type_should_fail() {
        let input: &[u8] = b"z\r\n...\r\n";
        assert_eq!(RespType::try_from(input), Err(Error::UnknownType('z')));
    }

    #[test]
    fn parse_simple_string() -> Result<(), Error> {
        let input: &[u8] = b"+OK\r\n";
        let result = RespType::try_from(input)?;

        assert_eq!(result, RespType::SimpleString("OK".into()));

        Ok(())
    }

    #[test]
    fn parse_bulk_string() -> Result<(), Error> {
        let input: &[u8] = b"$4\r\nRust\r\n";
        let result = RespType::try_from(input)?;

        assert_eq!(result, RespType::BulkString(b"Rust"[..].into()));

        Ok(())
    }

    #[test]
    fn invalid_u32() {
        let input: &[u8] = b"$h\r\nhello\r\n";
        assert_eq!(
            RespType::try_from(input),
            Err(Error::InvalidUnsigned32BitNumber)
        );
    }

    #[test]
    fn invalid_length() {
        let input: &[u8] = b"$2\r\nRust\r\n";
        assert_eq!(RespType::try_from(input), Err(Error::InvalidLength));
    }

    #[test]
    fn parse_array_same_type() -> Result<(), Error> {
        let input: &[u8] = b"*1\r\n$4\r\nPING\r\n";
        let result = RespType::try_from(input)?;

        assert_eq!(
            result,
            RespType::Array(VecDeque::from([RespType::BulkString(b"PING"[..].into())]))
        );

        Ok(())
    }

    #[test]
    fn parse_array_mixed_type() -> Result<(), Error> {
        let input: &[u8] = b"*2\r\n$4\r\nPING\r\n+OK\r\n";
        let result = RespType::try_from(input)?;

        assert_eq!(
            result,
            RespType::Array(VecDeque::from([
                RespType::BulkString(b"PING"[..].into()),
                RespType::SimpleString("OK".into())
            ]))
        );

        Ok(())
    }

    #[test]
    fn extra_bytes() {
        let input: &[u8] = b"+Sure\r\njunk...";
        assert_eq!(RespType::try_from(input), Err(Error::ExtraBytes));
    }

    #[test]
    fn invalid_utf8_in_simple_string() {
        // Invalid UTF-8 starting with + and finishing with \r\n
        let input: &[u8] = &[0x2B, 0xF0, 0x28, 0x8C, 0x28, 0x0D, 0x0A];
        assert_eq!(RespType::try_from(input), Err(Error::InvalidUTF8));
    }

    #[test]
    fn invalid_utf8_in_length() {
        // Invalid UTF-8 starting with $2 and finishing with \r\n
        let input: &[u8] = &[0x24, 0x32, 0xF0, 0x28, 0x8C, 0x28, 0x0D, 0x0A];
        assert_eq!(RespType::try_from(input), Err(Error::InvalidUTF8));
    }

    #[test]
    fn parse_simple_error() -> Result<(), Error> {
        let input: &[u8] = b"-ERR unknown command 'asdf'\r\n";
        let result = RespType::try_from(input)?;

        assert_eq!(
            result,
            RespType::SimpleError("ERR unknown command 'asdf'".into())
        );

        Ok(())
    }
}
