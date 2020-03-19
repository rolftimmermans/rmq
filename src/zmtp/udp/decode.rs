use bytes::{Buf, BytesMut};
use smallvec::SmallVec;
use tokio_util::codec::Decoder;

pub(super) use super::{super::decode::Error, frame::Frame, Zudp};

impl Decoder for Zudp {
    type Item = Frame;
    type Error = Error;

    fn decode(&mut self, buffer: &mut BytesMut) -> Result<Option<Frame>, Self::Error> {
        if buffer.is_empty() {
            return Err(Error::InvalidData);
        }

        let group_len = buffer.get_u8() as usize;
        if buffer.len() < group_len {
            return Err(Error::InvalidData);
        }

        let mut group = SmallVec::with_capacity(group_len);
        group.extend_from_slice(&buffer[..group_len]);
        buffer.advance(group_len);

        if buffer.len() > self.max_message_size {
            return Err(Error::InvalidFrameSize);
        }

        Ok(Some(Frame::Message {
            group,
            payload: buffer.split().freeze(),
        }))
    }
}
