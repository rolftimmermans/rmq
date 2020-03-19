use crate::sync::atomic::{AtomicU32, Ordering};

#[derive(Debug)]
pub(crate) struct Sequence {
    num: AtomicU32,
}

impl Default for Sequence {
    fn default() -> Self {
        Self {
            num: AtomicU32::new(rand::random()),
        }
    }
}

impl Sequence {
    pub(crate) fn next(&self) -> u32 {
        self.num.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_returns_sequential_numbers() {
        let seq = Sequence {
            num: AtomicU32::new(4),
        };
        assert_eq!(seq.next(), 4);
        assert_eq!(seq.next(), 5);
    }

    #[test]
    fn next_wraps_around() {
        let seq = Sequence {
            num: AtomicU32::new(0xffffffff),
        };
        assert_eq!(seq.next(), 0xffffffff);
        assert_eq!(seq.next(), 0);
    }
}
