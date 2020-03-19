use std::borrow::Cow;
use std::collections::HashMap;
use std::{io, str};

use bytes::{Buf, BytesMut};
use tokio_util::codec::Decoder;

use super::frame::{tag, Frame, Security, SmallBuf, SocketType};
use super::Zmtp;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Io,
    InvalidData,
    InvalidFrameSize,
    UnknownCommand,
    UnknownMechanism,
    UnknownSocketType,
}

macro_rules! ensure_capacity(
    { $buf:expr, $value:expr } => {
        {
            if $buf.len() < $value {
                $buf.reserve($value + 10 - $buf.len());
                return Ok(None);
            }
        }
    };
);

impl Decoder for Zmtp {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Frame>, Self::Error> {
        ensure_capacity!(buffer, 2);
        let marker = buffer[0];

        if marker == 0xff {
            ensure_capacity!(buffer, 64);
            return Ok(Some(decode_greeting(buffer)?));
        }

        if marker & 0b11111000 != 0 {
            // Reserved bytes are set
            return Err(Error::InvalidData);
        }

        let len;
        if marker & 0x02 == 0 {
            // Short message or command
            len = buffer[1] as usize;
            ensure_capacity!(buffer, len + 2);
            buffer.advance(2);
        } else {
            // Long message
            ensure_capacity!(buffer, 9);
            len = parse_u64(&buffer[1..9]) as usize;

            if len > self.max_message_size {
                return Err(Error::InvalidFrameSize);
            }

            ensure_capacity!(buffer, len + 9);
            buffer.advance(9);
        }

        let mut payload = buffer.split_to(len);
        if marker & 0x04 == 0 {
            // Message
            Ok(Some(Frame::Message {
                more: marker & 0x01 == 1,
                payload: payload.freeze(),
            }))
        } else {
            // Command
            if len == 0 {
                return Err(Error::InvalidData);
            }

            let cmd_len = payload.get_u8() as usize;
            if cmd_len >= len {
                return Err(Error::InvalidData);
            }

            let mut cmd_name = SmallBuf::with_capacity(cmd_len);
            cmd_name.extend_from_slice(&payload[..cmd_len]);
            payload.advance(cmd_len);

            let cmd = match cmd_name.as_ref() {
                tag::READY => decode_ready(&mut payload)?,

                tag::ERROR => Frame::Error {
                    reason: payload.bytes().into(),
                },

                tag::PING => {
                    if cmd_len + 2 >= len {
                        return Err(Error::InvalidData);
                    }

                    Frame::Ping {
                        ttl: payload.get_u16(),
                        context: payload.bytes().into(),
                    }
                }

                tag::PONG => Frame::Pong {
                    context: payload.bytes().into(),
                },

                tag::SUBSCRIBE => Frame::Subscribe {
                    group: payload.bytes().into(),
                },

                tag::CANCEL => Frame::Cancel {
                    group: payload.bytes().into(),
                },

                tag::JOIN => Frame::Join {
                    group: payload.bytes().into(),
                },

                tag::LEAVE => Frame::Leave {
                    group: payload.bytes().into(),
                },

                _ => {
                    return Err(Error::UnknownCommand);
                }
            };

            Ok(Some(cmd))
        }
    }
}

fn decode_greeting(buffer: &mut BytesMut) -> Result<Frame, Error> {
    if buffer[9] != 0x7f {
        return Err(Error::InvalidData);
    }

    let version = (buffer[10], buffer[11]);
    let security = Security::from_bytes(&buffer[12..32]).ok_or(Error::UnknownMechanism)?;

    if buffer[32] != 0 {
        todo!()
    }

    buffer.advance(64);
    buffer.reserve(10);
    return Ok(Frame::Greeting { version, security });
}

fn decode_ready(buffer: &mut BytesMut) -> Result<Frame, Error> {
    let mut properties = HashMap::<Cow<'static, _>, _>::default();
    let mut socket_type = None;

    while !buffer.is_empty() {
        let key_len = buffer.get_u8() as usize;
        if key_len > buffer.len() {
            return Err(Error::InvalidData);
        }

        let key = parse_key(&buffer[..key_len]).ok_or(Error::InvalidData)?;

        buffer.advance(key_len);

        if buffer.is_empty() {
            return Err(Error::InvalidData);
        }

        let val_len = buffer.get_u32() as usize;
        if val_len > buffer.len() {
            return Err(Error::InvalidData);
        }

        if key == tag::SOCKET_TYPE {
            socket_type =
                Some(SocketType::from_bytes(&buffer[..val_len]).ok_or(Error::UnknownSocketType)?)
        } else {
            let val = buffer.split_to(val_len).freeze();
            properties.insert(key.to_owned().into(), val.to_vec());
        }

        buffer.advance(val_len);
    }

    match socket_type {
        None => Err(Error::InvalidData),
        Some(socket_type) => Ok(Frame::Ready {
            socket_type,
            properties,
        }),
    }
}

#[inline]
fn parse_key(buffer: &[u8]) -> Option<Cow<'static, str>> {
    Some(match str::from_utf8(buffer).ok()? {
        tag::RESOURCE => Cow::Borrowed(tag::RESOURCE),
        tag::IDENTITY => Cow::Borrowed(tag::IDENTITY),
        tag::SOCKET_TYPE => Cow::Borrowed(tag::SOCKET_TYPE),
        key => Cow::Owned(key.to_owned()),
    })
}

#[inline]
fn parse_u64(buffer: &[u8]) -> u64 {
    let mut lenbuf = [0u8; 8];
    lenbuf.copy_from_slice(&buffer[..8]);
    u64::from_be_bytes(lenbuf)
}

impl From<io::Error> for Error {
    fn from(_cause: io::Error) -> Self {
        Self::Io
    }
}
