mod decode;
mod encode;
mod frame;

#[derive(Debug)]
pub(crate) struct Zudp {
    pub max_message_size: usize,
}

impl Default for Zudp {
    fn default() -> Self {
        Self {
            max_message_size: 8192,
        }
    }
}

use tokio::net::UdpSocket;
use tokio_util::udp;

pub(crate) use frame::*;

pub(crate) type Framed = udp::UdpFramed<Zudp>;

pub(crate) fn frame(socket: UdpSocket) -> Framed {
    Framed::new(socket, Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{Bytes, BytesMut};
    use smallvec::smallvec;
    use tokio_util::codec::{Decoder, Encoder};

    macro_rules! assert_troundtrips {
        ( $data:expr, $frame:expr ) => {{
            let data = $data;
            let frame = $frame;
            let mut codec = Zudp::default();

            let mut buf = BytesMut::from(&data[..]);
            let decoded = codec.decode(&mut buf).expect("decode").expect("empty");
            assert_eq!($frame, decoded);

            let mut buf = BytesMut::with_capacity(data.len() + 1);
            codec.encode(decoded, &mut buf).expect("encode");
            assert_eq!(&data[..], &buf[..data.len()]);
        }};
    }

    #[test]
    fn roundtrips_frame() {
        assert_troundtrips!(
            b"\x05greethello world",
            Frame::Message {
                group: (&b"greet"[..]).into(),
                payload: Bytes::from(&b"hello world"[..]),
            }
        );
    }

    #[test]
    fn roundtrips_frame_with_empty_group() {
        assert_troundtrips!(
            b"\0hello world",
            Frame::Message {
                group: smallvec![],
                payload: Bytes::from(&b"hello world"[..]),
            }
        );
    }

    #[test]
    fn roundtrips_frame_with_empty_message() {
        assert_troundtrips!(
            b"\x0bhello world",
            Frame::Message {
                group: (&b"hello world"[..]).into(),
                payload: Bytes::new(),
            }
        );
    }

    #[test]
    fn decode_fails_on_invalid_messages() {
        let invalid = vec![&b""[..], &b"\x02\0"[..]];

        for msg in invalid {
            let mut decoder = Zudp::default();
            let mut buf = BytesMut::from(msg);
            assert_eq!(
                super::decode::Error::InvalidData,
                decoder.decode(&mut buf).expect_err("success")
            );
        }
    }
}
