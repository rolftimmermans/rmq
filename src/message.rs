use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::net::SocketAddr;
use std::num::NonZeroU8;
use std::str;

use crate::sync::Arc;
use crate::zmtp::SocketType;

#[derive(Default, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Payload {
    inner: bytes::Bytes,
}

impl<T: Into<bytes::Bytes>> From<T> for Payload {
    fn from(payload: T) -> Self {
        Self {
            inner: payload.into(),
        }
    }
}

impl fmt::Debug for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl Payload {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.inner
    }

    pub fn into_bytes(self) -> bytes::Bytes {
        self.inner
    }
}

#[derive(Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Group {
    buffer: [u8; 15],
    length: u8,
}

#[derive(Debug, Default)]
pub struct GroupError;

impl Group {
    fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.length as usize]
    }
}

impl str::FromStr for Group {
    type Err = GroupError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        string.as_bytes().try_into()
    }
}

impl TryFrom<&[u8]> for Group {
    type Error = GroupError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() > 15 {
            return Err(GroupError);
        }

        if value.iter().any(|&b| b == 0) {
            return Err(GroupError);
        }

        let mut group = Group::default();
        group.buffer[..value.len()].copy_from_slice(value);
        group.length = value.len() as u8;
        Ok(group)
    }
}

impl fmt::Display for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"")?;
        for &b in self.as_bytes() {
            // https://doc.rust-lang.org/reference/tokens.html#byte-escapes
            if b == b'\n' {
                write!(f, "\\n")?;
            } else if b == b'\r' {
                write!(f, "\\r")?;
            } else if b == b'\t' {
                write!(f, "\\t")?;
            } else if b == b'\\' || b == b'"' {
                write!(f, "\\{}", b as char)?;
            } else if b == b'\0' {
                write!(f, "\\0")?;
            // ASCII printable
            } else if b >= 0x20 && b < 0x7f {
                write!(f, "{}", b as char)?;
            } else {
                write!(f, "\\x{:02x}", b)?;
            }
        }
        write!(f, "\"")?;
        Ok(())
    }
}

impl fmt::Debug for Group {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Route {
    pub(crate) id: u32,
}

impl fmt::Display for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08x}", self.id)
    }
}

impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Message {
    pub(crate) payload: Payload,
    pub(crate) group: Group,
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.payload, f)
    }
}

impl Message {
    pub fn as_bytes(&self) -> &[u8] {
        self.payload.as_bytes()
    }

    pub fn into_bytes(self) -> bytes::Bytes {
        self.payload.into_bytes()
    }
}

pub trait IntoMessage: Sized {
    fn into_payload(self) -> Payload;

    fn into_message(self) -> Message {
        self.into_message_with_group(Default::default())
    }

    fn into_message_with_group(self, group: Group) -> Message {
        Message {
            payload: self.into_payload(),
            group,
        }
    }
}

impl<T: Into<Payload>> IntoMessage for T {
    fn into_payload(self) -> Payload {
        self.into()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Info {
    pub(crate) peer_address: Option<SocketAddr>,
    pub(crate) identity: Option<Vec<u8>>,
    pub(crate) resource: Option<Vec<u8>>,
    pub(crate) custom: HashMap<Cow<'static, str>, Vec<u8>>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Envelope<T> {
    pub(crate) info: Arc<Info>,
    pub route: Route,
    pub message: T,
}

impl fmt::Debug for Envelope<Message> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Envelope")
            .field("route", &self.route)
            .field("message", &self.message)
            .finish()
    }
}

impl Envelope<Message> {
    pub fn peer_address(&self) -> Option<&SocketAddr> {
        self.info.peer_address.as_ref()
    }

    pub fn peer_identity(&self) -> Option<&[u8]> {
        self.info.identity.as_ref().map(Vec::as_ref)
    }

    pub fn resource(&self) -> Option<&str> {
        use bytes::Buf;
        str::from_utf8(self.info.resource.as_ref()?.as_ref()).ok()
    }

    pub fn meta(&self, key: &str) -> Option<&[u8]> {
        self.info.custom.get(key).map(Vec::as_ref)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.message.as_bytes()
    }

    pub fn into_bytes(self) -> bytes::Bytes {
        self.message.into_bytes()
    }
}

impl<Rhs: AsRef<[u8]>> PartialEq<Rhs> for Envelope<Message> {
    fn eq(&self, other: &Rhs) -> bool {
        self.message.as_bytes() == other.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::*;

    #[test]
    fn messages_fit_in_cache_line() {
        assert_le!(std::mem::size_of::<Envelope<Message>>(), 64);
    }

    #[test]
    fn group_parse_parses_valid_group() {
        let group = "hello world!".parse::<Group>();
        assert_ok_eq!(
            group,
            Group {
                buffer: [
                    b'h', b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd', b'!', 0, 0, 0
                ],
                length: 12
            }
        );
    }

    #[test]
    fn group_parse_fails_invalid_groups() {
        assert_err!("hello fine world".parse::<Group>());
        assert_err!("hello\0world!".parse::<Group>());
    }

    #[test]
    fn group_display() {
        let group = "hello\x01world!".parse::<Group>().expect("parse");
        assert_eq!("b\"hello\\x01world!\"", group.to_string());
    }
}
