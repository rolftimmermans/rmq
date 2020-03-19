use bytes::Bytes;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Security {
    Null,
    Plain,
    Curve,
}

impl Default for Security {
    fn default() -> Self {
        Self::Null
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SocketType {
    // New socket types
    CLIENT,
    SERVER,
    RADIO,
    DISH,
    SCATTER,
    GATHER,
    PEER,

    // Old socket types
    REQ,
    REP,
    DEALER,
    ROUTER,
    PUB,
    XPUB,
    SUB,
    XSUB,
    PUSH,
    PULL,
    PAIR,
}

impl Default for SocketType {
    fn default() -> Self {
        Self::CLIENT
    }
}

pub(crate) type SmallBuf = SmallVec<[u8; 16]>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Frame {
    Greeting {
        version: (u8, u8),
        security: Security,
    },

    Message {
        more: bool,
        payload: Bytes, // TODO use inline smallvec?
    },

    Ready {
        socket_type: SocketType,
        properties: HashMap<Cow<'static, str>, Vec<u8>>, // TODO use inline smallvec?
    },

    Error {
        reason: SmallVec<[u8; 32]>, // max 255 bytes
    },

    Ping {
        ttl: u16,
        context: SmallVec<[u8; 16]>, // max 16 bytes
    },

    Pong {
        context: SmallVec<[u8; 16]>, // max 16 bytes
    },

    Subscribe {
        group: SmallVec<[u8; 32]>, // any length
    },

    Cancel {
        group: SmallVec<[u8; 32]>, // any length
    },

    Join {
        group: SmallVec<[u8; 16]>, // max 255 bytes, but 15 in practice TODO: use nonzero u8
    },

    Leave {
        group: SmallVec<[u8; 16]>, // max 255 bytes, but 15 in practice TODO: use nonzero u8
    },
}

impl Security {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<Security> {
        match bytes {
            tag::NULL => Some(Security::Null),
            tag::PLAIN => Some(Security::Plain),
            tag::CURVE => Some(Security::Curve),
            _ => None,
        }
    }

    pub(crate) fn as_bytes(self) -> &'static [u8] {
        match self {
            Security::Null => tag::NULL,
            Security::Plain => tag::PLAIN,
            Security::Curve => tag::CURVE,
        }
    }
}

impl SocketType {
    pub(crate) fn from_bytes(bytes: &[u8]) -> Option<SocketType> {
        match bytes {
            tag::REQ => Some(SocketType::REQ),
            tag::REP => Some(SocketType::REP),
            tag::DEALER => Some(SocketType::DEALER),
            tag::ROUTER => Some(SocketType::ROUTER),
            tag::PUB => Some(SocketType::PUB),
            tag::XPUB => Some(SocketType::XPUB),
            tag::SUB => Some(SocketType::SUB),
            tag::XSUB => Some(SocketType::XSUB),
            tag::PUSH => Some(SocketType::PUSH),
            tag::PULL => Some(SocketType::PULL),
            tag::PAIR => Some(SocketType::PAIR),
            tag::CLIENT => Some(SocketType::CLIENT),
            tag::SERVER => Some(SocketType::SERVER),
            tag::RADIO => Some(SocketType::RADIO),
            tag::DISH => Some(SocketType::DISH),
            tag::SCATTER => Some(SocketType::SCATTER),
            tag::GATHER => Some(SocketType::GATHER),
            tag::PEER => Some(SocketType::PEER),
            _ => None,
        }
    }

    pub(crate) fn as_bytes(self) -> &'static [u8] {
        match self {
            SocketType::REQ => tag::REQ,
            SocketType::REP => tag::REP,
            SocketType::DEALER => tag::DEALER,
            SocketType::ROUTER => tag::ROUTER,
            SocketType::PUB => tag::PUB,
            SocketType::XPUB => tag::XPUB,
            SocketType::SUB => tag::SUB,
            SocketType::XSUB => tag::XSUB,
            SocketType::PUSH => tag::PUSH,
            SocketType::PULL => tag::PULL,
            SocketType::PAIR => tag::PAIR,
            SocketType::CLIENT => tag::CLIENT,
            SocketType::SERVER => tag::SERVER,
            SocketType::RADIO => tag::RADIO,
            SocketType::DISH => tag::DISH,
            SocketType::SCATTER => tag::SCATTER,
            SocketType::GATHER => tag::GATHER,
            SocketType::PEER => tag::PEER,
        }
    }
}

pub(crate) mod tag {
    pub const SIGNATURE: &[u8] = b"\xff\0\0\0\0\0\0\0\0\x7f";

    pub const READY: &[u8] = b"READY";
    pub const ERROR: &[u8] = b"ERROR";
    pub const PING: &[u8] = b"PING";
    pub const PONG: &[u8] = b"PONG";
    pub const SUBSCRIBE: &[u8] = b"SUBSCRIBE";
    pub const CANCEL: &[u8] = b"CANCEL";
    pub const JOIN: &[u8] = b"JOIN";
    pub const LEAVE: &[u8] = b"LEAVE";

    pub const NULL: &[u8] = b"NULL\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
    pub const PLAIN: &[u8] = b"PLAIN\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
    pub const CURVE: &[u8] = b"CURVE\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

    // Old socket types
    pub const REQ: &[u8] = b"REQ";
    pub const REP: &[u8] = b"REP";
    pub const DEALER: &[u8] = b"DEALER";
    pub const ROUTER: &[u8] = b"ROUTER";
    pub const PUB: &[u8] = b"PUB";
    pub const XPUB: &[u8] = b"XPUB";
    pub const SUB: &[u8] = b"SUB";
    pub const XSUB: &[u8] = b"XSUB";
    pub const PUSH: &[u8] = b"PUSH";
    pub const PULL: &[u8] = b"PULL";
    pub const PAIR: &[u8] = b"PAIR";

    // New socket types
    pub const CLIENT: &[u8] = b"CLIENT";
    pub const SERVER: &[u8] = b"SERVER";
    pub const RADIO: &[u8] = b"RADIO";
    pub const DISH: &[u8] = b"DISH";
    pub const SCATTER: &[u8] = b"SCATTER";
    pub const GATHER: &[u8] = b"GATHER";
    pub const PEER: &[u8] = b"PEER";

    pub const SOCKET_TYPE: &str = "Socket-Type";
    pub const IDENTITY: &str = "Identity";
    pub const RESOURCE: &str = "Resource";
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::*;

    #[test]
    fn frame_fits_in_cache_line() {
        assert_le!(std::mem::size_of::<Frame>(), 64);
    }
}
