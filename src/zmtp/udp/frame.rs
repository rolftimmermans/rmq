use bytes::Bytes;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Frame {
    Message {
        group: SmallVec<[u8; 16]>, // max 255 bytes, but 15 in practice TODO: use nonzero u8
        payload: Bytes,
    },
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
