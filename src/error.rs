use std::{fmt, io};

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    RoutingError,
    PermissionDenied,
    TransportUnknown,
    TransportUnavailable,
    AddressInUse,
    AddressInvalid,
    AddressNotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
        // match self {
        //     Self::PermissionDenied => write!(f, "permission denied"),
        //     _ => write!(f, "other error"),
        // }
    }
}

impl From<io::Error> for Error {
    fn from(cause: io::Error) -> Self {
        match cause.kind() {
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            io::ErrorKind::AddrInUse => Self::AddressInUse,
            err => panic!("io error {:?}", err),
        }
    }
}
