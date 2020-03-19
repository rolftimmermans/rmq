use bytes::{BufMut, BytesMut};
use tokio_util::codec::Encoder;

pub(super) use super::{super::decode::Error, frame::Frame, Zudp};

impl Encoder<Frame> for Zudp {
    type Error = Error;

    fn encode(&mut self, frame: Frame, buffer: &mut BytesMut) -> Result<(), Error> {
        match frame {
            Frame::Message { group, payload } => {
                buffer.put_u8(group.len() as u8);
                buffer.put_slice(&group);
                buffer.put_slice(&payload);
            }
        };

        Ok(())
    }
}
