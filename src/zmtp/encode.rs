use std::io;

use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

use super::frame::{tag, Frame};
use super::Zmtp;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io,
}

impl Encoder<Frame> for Zmtp {
    type Error = Error;

    fn encode(&mut self, frame: Frame, buffer: &mut BytesMut) -> Result<(), Error> {
        // https://github.com/hyperium/hyper/blob/master/src/common/buf.rs
        match frame {
            Frame::Greeting { version, security } => {
                buffer.put_slice(tag::SIGNATURE);
                buffer.put_u8(version.0);
                buffer.put_u8(version.1);
                buffer.put_slice(security.as_bytes());
                buffer.resize(64, 0);
            }

            Frame::Message { payload, more } => {
                if payload.len() > 255 {
                    buffer.put_u8(2 | more as u8);
                    buffer.put_u64(payload.len() as u64);
                } else {
                    buffer.put_u8(more as u8);
                    buffer.put_u8(payload.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&payload);
            }

            Frame::Ready {
                socket_type,
                properties,
            } => {
                let mut data = BytesMut::new();
                data.put_u8(tag::READY.len() as u8);
                data.put_slice(tag::READY);

                data.put_u8(tag::SOCKET_TYPE.len() as u8);
                data.put(tag::SOCKET_TYPE.as_bytes());
                data.put_u32(socket_type.as_bytes().len() as u32);
                data.put_slice(socket_type.as_bytes());

                for (key, val) in properties {
                    data.put_u8(key.len() as u8);
                    data.put(key.as_bytes());
                    data.put_u32(val.len() as u32);
                    data.put_slice(&val);
                }

                if data.len() > 255 {
                    buffer.put_u8(6);
                    buffer.put_u64(data.len() as u64);
                } else {
                    buffer.put_u8(4);
                    buffer.put_u8(data.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&data);
            }

            Frame::Join { group } => {
                let mut data = BytesMut::new();
                data.put_u8(tag::JOIN.len() as u8);
                data.put_slice(tag::JOIN);
                data.put_slice(&group);

                if data.len() > 255 {
                    buffer.put_u8(6);
                    buffer.put_u64(data.len() as u64);
                } else {
                    buffer.put_u8(4);
                    buffer.put_u8(data.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&data);
            }

            Frame::Leave { group } => {
                let mut data = BytesMut::new();
                data.put_u8(tag::LEAVE.len() as u8);
                data.put_slice(tag::LEAVE);
                data.put_slice(&group);

                if data.len() > 255 {
                    buffer.put_u8(6);
                    buffer.put_u64(data.len() as u64);
                } else {
                    buffer.put_u8(4);
                    buffer.put_u8(data.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&data);
            }

            Frame::Ping { ttl, context } => {
                buffer.put_u8(4);
                buffer.put_u8(3 + (tag::PING.len() + context.len()) as u8);
                buffer.put_u8(tag::PING.len() as u8);
                buffer.put_slice(tag::PING);
                buffer.put_u16(ttl);
                buffer.put_slice(&context);
            }

            Frame::Pong { context } => {
                buffer.put_u8(4);
                buffer.put_u8(1 + (tag::PONG.len() + context.len()) as u8);
                buffer.put_u8(tag::PONG.len() as u8);
                buffer.put_slice(tag::PONG);
                buffer.put_slice(&context);
            }

            Frame::Subscribe { group } => {
                let mut data = BytesMut::new();
                data.put_u8(tag::SUBSCRIBE.len() as u8);
                data.put_slice(tag::SUBSCRIBE);
                data.put_slice(&group);

                if data.len() > 255 {
                    buffer.put_u8(6);
                    buffer.put_u64(data.len() as u64);
                } else {
                    buffer.put_u8(4);
                    buffer.put_u8(data.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&data);
            }

            Frame::Cancel { group } => {
                let mut data = BytesMut::new();
                data.put_u8(tag::CANCEL.len() as u8);
                data.put_slice(tag::CANCEL);
                data.put_slice(&group);

                if data.len() > 255 {
                    buffer.put_u8(6);
                    buffer.put_u64(data.len() as u64);
                } else {
                    buffer.put_u8(4);
                    buffer.put_u8(data.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&data);
            }

            Frame::Error { reason } => {
                let mut data = BytesMut::new();
                data.put_u8(tag::ERROR.len() as u8);
                data.put_slice(tag::ERROR);
                data.put_slice(&reason);

                if data.len() > 255 {
                    buffer.put_u8(6);
                    buffer.put_u64(data.len() as u64);
                } else {
                    buffer.put_u8(4);
                    buffer.put_u8(data.len() as u8);
                }

                // TODO avoid copy by reusing same buffer.
                buffer.put_slice(&data);
            }
        };

        Ok(())
    }
}

impl From<io::Error> for Error {
    fn from(_cause: io::Error) -> Self {
        Self::Io
    }
}

#[cfg(test)]
mod tests {
    use super::super::frame::SocketType;
    use super::*;

    use bytes::Buf;

    #[test]
    fn encodes_ready_zmtp_spec1() {
        let frame = Frame::Ready {
            socket_type: SocketType::CLIENT,
            properties: map!("Identity" => b""),
        };

        let mut encoder = Zmtp::default();
        let mut buffer = BytesMut::new();
        encoder.encode(frame, &mut buffer).unwrap();
        assert_eq!(
            buffer.bytes(),
            &b"\x04\x29\x05READY\x0bSocket-Type\0\0\0\x06CLIENT\x08Identity\0\0\0\0"[..]
        );
    }

    #[test]
    fn encodes_ready_zmtp_spec2() {
        let frame = Frame::Ready {
            socket_type: SocketType::SERVER,
            properties: map!(),
        };

        let mut encoder = Zmtp::default();
        let mut buffer = BytesMut::new();
        encoder.encode(frame, &mut buffer).unwrap();
        assert_eq!(
            buffer.bytes(),
            &b"\x04\x1c\x05READY\x0bSocket-Type\0\0\0\x06SERVER"[..]
        );
    }

    // #[async_std::test]
    // async fn streams_ready() {
    //     let ready = Ready {
    //         socket_type: SocketType::DEALER,
    //         meta: map!(),
    //     };

    //     let frame = Command(Command::READY(ready));
    //     let stream = Box::pin(futures::stream::once(async { frame }));
    //     let mut encoder = Encoder::new(Zmtp, stream);
    //     let buf = encoder.next().await.expect("encoded frame").expect("no encoding errors");
    //     assert_eq!(buffer.bytes(), &b"\x04\x1c\x05READY\x0bSocket-Type\0\0\0\x06DEALER"[..]);
    // }
}
