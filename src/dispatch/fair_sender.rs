use futures::future::poll_fn;
use futures::task::AtomicWaker;
use std::task::{Context, Poll};

use super::{Delivery, Register, Sender};
use crate::sync::{Arc, Mutex};
use crate::{Error, Message, Route};

#[derive(Debug, Default)]
pub(crate) struct FairSender {
    peers: Vec<(Route, Sender)>,
    next: usize,
    waker: AtomicWaker,
}

impl Register<Sender> for FairSender {
    fn insert(&mut self, id: Route, peer: Sender) {
        self.peers.push((id, peer));
        self.waker.wake();
    }

    fn remove(&mut self, ref id: Route) {
        for (idx, (key, ..)) in self.peers.iter().enumerate() {
            if key == id {
                self.peers.swap_remove(idx);
                return;
            }
        }

        panic!("removing unknown peer");
    }
}

impl FairSender {
    pub(crate) fn poll_send(
        &mut self,
        message: &mut Option<Message>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        let mut i = 0;

        while i != self.peers.len() {
            let idx = (self.next + i) % self.peers.len();
            let (_, ref mut peer) = self.peers[idx];

            if peer.tx.poll_ready(cx).is_ready() {
                let message = message.take().expect("message taken before send");
                if let Err(..) = peer.tx.try_send(Delivery::Message(message)) {
                    panic!("session was dropped")
                }

                self.next = idx + 1;
                return Poll::Ready(Ok(()));
            }

            i += 1;
        }

        // Request to be woken up if new peers are added.
        self.waker.register(cx.waker());

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::Peer;
    use crate::socket::Options;
    use claim::*;
    use futures::{Future, FutureExt};

    const OPTIONS: Options = Options {
        outgoing_queue_size: 2,
        incoming_queue_size: 2,
        max_reconnect_interval: std::time::Duration::from_micros(1),
        heartbeat_timeout: std::time::Duration::from_micros(1),
    };

    #[test]
    fn send_queues_messages_to_next_peer() {
        let mut sender = FairSender::default();
        let (peer1, mut pipe1) = Peer::create(&OPTIONS);
        let (peer2, mut pipe2) = Peer::create(&OPTIONS);

        assert_pending!(sender.poll_send(&mut Some(msg!(1)), cx!()));

        sender.insert(peer1.id, peer1.tx);
        sender.insert(peer2.id, peer2.tx);

        assert_ready_eq!(sender.poll_send(&mut Some(msg!(1)), cx!()), Ok(()));
        assert_ready_eq!(sender.poll_send(&mut Some(msg!(2)), cx!()), Ok(()));
        assert_ready_eq!(sender.poll_send(&mut Some(msg!(3)), cx!()), Ok(()));
        assert_ready_eq!(sender.poll_send(&mut Some(msg!(4)), cx!()), Ok(()));
        assert_pending!(sender.poll_send(&mut Some(msg!()), cx!()));

        assert_ok_eq!(pipe1.rx.try_recv(), delivery!(1));
        assert_ok_eq!(pipe2.rx.try_recv(), delivery!(2));
        assert_ok_eq!(pipe1.rx.try_recv(), delivery!(3));
        assert_ok_eq!(pipe2.rx.try_recv(), delivery!(4));

        sender.remove(peer2.id);

        assert_ready_eq!(sender.poll_send(&mut Some(msg!(5)), cx!()), Ok(()));
        assert_ready_eq!(sender.poll_send(&mut Some(msg!(6)), cx!()), Ok(()));

        assert_ok_eq!(pipe1.rx.try_recv(), delivery!(5));
        assert_ok_eq!(pipe1.rx.try_recv(), delivery!(6));
    }

    #[test]
    #[should_panic(expected = "session was dropped")]
    fn send_panics_if_session_queue_is_dropped() {
        let mut sender = FairSender::default();
        let (peer, pipe) = Peer::create(&OPTIONS);

        let id = pipe.id;
        drop(pipe);

        sender.insert(peer.id, peer.tx);
        let _ = sender.poll_send(&mut Some(msg!(1)), cx!());
    }
}
