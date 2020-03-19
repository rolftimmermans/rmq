mod decode;
mod encode;
mod frame;
pub(crate) mod udp;

use futures::{SinkExt, StreamExt};
use std::borrow::Cow;
use std::collections::HashMap;
use tokio::io::{AsyncRead, AsyncWrite};

use tokio_util::codec;

pub(crate) use frame::*;
pub(crate) type Framed<T> = codec::Framed<T, Zmtp>;

#[derive(Debug)]
pub(crate) struct Zmtp {
    pub max_message_size: usize,
}

impl Default for Zmtp {
    fn default() -> Self {
        Self {
            #[cfg(test)]
            max_message_size: 1 << 24,

            #[cfg(not(test))]
            max_message_size: 1 << 32,
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub(crate) struct Params {
    pub socket_type: SocketType,
    pub security: Security,
}

#[derive(Default, Debug, Clone)]
pub(crate) struct Info {
    pub socket_type: SocketType,
    pub properties: HashMap<Cow<'static, str>, Vec<u8>>,
}

pub(crate) fn frame<T: AsyncRead + AsyncWrite + Unpin>(transport: T) -> Framed<T> {
    Framed::new(transport, Default::default())
}

pub(crate) async fn connect<T: AsyncRead + AsyncWrite + Unpin>(
    transport: &mut Framed<T>,
    params: &Params,
) -> Result<Info, Error> {
    transport
        .send(Frame::Greeting {
            version: (3, 1),
            security: params.security,
        })
        .await?;

    let (_version, _security) = match transport.next().await {
        Some(Ok(Frame::Greeting { version, security })) => (version, security),
        Some(Ok(_frame)) => return Err(Error::UnexpectedFrame),
        Some(Err(err)) => return Err(err.into()),
        None => return Err(Error::Disconnected),
    };

    transport
        .send(Frame::Ready {
            socket_type: params.socket_type,
            properties: Default::default(),
        })
        .await?;

    match transport.next().await {
        Some(Ok(Frame::Ready {
            socket_type,
            properties,
        })) => Ok(Info {
            socket_type,
            properties,
        }),

        Some(Ok(Frame::Error { .. })) => Err(Error::HandshakeFailed),
        Some(Ok(_frame)) => Err(Error::UnexpectedFrame),
        Some(Err(err)) => Err(err.into()),
        None => Err(Error::Disconnected),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Error {
    Disconnected,
    UnexpectedFrame,
    HandshakeFailed,
    Encode(encode::Error),
    Decode(decode::Error),
}

impl From<encode::Error> for Error {
    fn from(cause: encode::Error) -> Self {
        Self::Encode(cause)
    }
}

impl From<decode::Error> for Error {
    fn from(cause: decode::Error) -> Self {
        Self::Decode(cause)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Bytes, BytesMut};
    use smallvec::smallvec;
    use tokio_util::codec::{Decoder, Encoder};

    macro_rules! assert_roundtrips {
        ( $data:expr, $frame:expr ) => {{
            let data = $data;
            let mut codec = Zmtp::default();

            // Partial decode
            for i in 0..data.len() {
                let mut buf = BytesMut::from(&data[..i]);
                assert_eq!(None, codec.decode(&mut buf).unwrap());
            }

            // Full decode
            let mut buf = BytesMut::from([&data[..], &b"\xff"[..]].concat().as_slice());
            let decoded = codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!($frame, decoded);

            // Next frame
            assert_eq!(None, codec.decode(&mut buf).unwrap());

            // Encode
            let mut buf = BytesMut::with_capacity(data.len() + 1);
            codec.encode(decoded, &mut buf).unwrap();
            assert_eq!(&data[..], &buf[..data.len()]);
        }};
    }

    #[test]
    fn roundtrips_greeting() {
        assert_roundtrips!(
            b"\xff\0\0\0\0\0\0\0\0\x7f\x03\x01NULL\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            Frame::Greeting {
                version: (3, 1),
                security: Security::Null,
            }
        );
    }

    #[test]
    fn roundtrips_short_message() {
        assert_roundtrips!(
            b"\x01\x0cHello world!",
            Frame::Message {
                more: true,
                payload: Bytes::from(&b"Hello world!"[..]),
            }
        );
    }

    #[test]
    fn roundtrips_long_message() {
        assert_roundtrips!(
            b"\x03\0\0\0\0\0\0\x01\x10Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    ",
            Frame::Message {
                more: true,
                payload: Bytes::from(&b"Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    Hello world!    "[..]),
            }
        );
    }

    #[test]
    fn roundtrips_ready_zmtp_spec1() {
        assert_roundtrips!(
            b"\x04\x29\x05READY\x0bSocket-Type\0\0\0\x06CLIENT\x08Identity\0\0\0\0",
            Frame::Ready {
                socket_type: SocketType::CLIENT,
                properties: map!("Identity" => b""),
            }
        );
    }

    #[test]
    fn roundtrips_ready_zmtp_spec2() {
        assert_roundtrips!(
            b"\x04\x1c\x05READY\x0bSocket-Type\0\0\0\x06SERVER",
            Frame::Ready {
                socket_type: SocketType::SERVER,
                properties: map!(),
            }
        );
    }

    #[test]
    fn roundtrips_error() {
        assert_roundtrips!(
            b"\x04\x0a\x05ERROROops",
            Frame::Error {
                reason: (&b"Oops"[..]).into(),
            }
        );
    }

    #[test]
    fn roundtrips_join() {
        assert_roundtrips!(
            b"\x04\x06\x04JOINa",
            Frame::Join {
                group: (&b"a"[..]).into(),
            }
        );
    }

    #[test]
    fn roundtrips_leave() {
        assert_roundtrips!(
            b"\x04\x07\x05LEAVEa",
            Frame::Leave {
                group: (&b"a"[..]).into(),
            }
        );
    }

    #[test]
    fn roundtrips_ping() {
        assert_roundtrips!(
            b"\x04\x08\x04PING\x01\x7fa",
            Frame::Ping {
                ttl: 383,
                context: (&b"a"[..]).into(),
            }
        );
    }

    #[test]
    fn roundtrips_pong() {
        assert_roundtrips!(
            b"\x04\x06\x04PONGa",
            Frame::Pong {
                context: (&b"a"[..]).into(),
            }
        );
    }

    #[test]
    fn roundtrips_subscribe() {
        assert_roundtrips!(
            b"\x04\x0b\x09SUBSCRIBEa",
            Frame::Subscribe {
                group: (&b"a"[..]).into(),
            }
        );
    }

    #[test]
    fn roundtrips_cancel() {
        assert_roundtrips!(
            b"\x04\x08\x06CANCELa",
            Frame::Cancel {
                group: (&b"a"[..]).into(),
            }
        );
    }

    #[test]
    fn decode_fails_on_invalid_messages() {
        let invalid = vec![
            &b"\x04\0"[..],
            &b"\x04\x01\x01"[..],
            &b"\x04\x06\x04PING\0\x7f %"[..],
            &b"\x04\x07\x05READY\0\0"[..],
        ];

        for msg in invalid {
            let mut decoder = Zmtp::default();
            let mut buf = BytesMut::from(msg);
            assert_eq!(
                decode::Error::InvalidData,
                decoder.decode(&mut buf).expect_err("success")
            );
        }
    }

    #[test]
    fn decode_fails_on_too_large_frames() {
        let invalid = vec![&b"\x06\x7f\xff\xff\xff\xff\xff\xff\xff"[..]];

        for msg in invalid {
            let mut decoder = Zmtp::default();
            let mut buf = BytesMut::from(msg);
            assert_eq!(
                decode::Error::InvalidFrameSize,
                decoder.decode(&mut buf).expect_err("success")
            );
        }
    }
}
