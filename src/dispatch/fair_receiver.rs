use futures::future::poll_fn;
use futures::task::AtomicWaker;
use std::collections::HashSet;
use std::task::{Context, Poll};

use super::{Delivery, Receiver, Register};
use crate::message::Info;
use crate::{Envelope, Error, Group, Message, Route};

#[derive(Debug, Default)]
pub(crate) struct FairReceiver {
    pub(super) peers: Vec<(Route, Receiver)>,
    next: usize,
    waker: AtomicWaker,
}

impl Register<Receiver> for FairReceiver {
    fn insert(&mut self, id: Route, peer: Receiver) {
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

lazy_static::lazy_static! {
    static ref NO_INFO: crate::sync::Arc<Info> = Default::default();
}

impl FairReceiver {
    pub(crate) fn poll_recv(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Envelope<Message>, Error>> {
        let mut i = 0;

        while i != self.peers.len() {
            let idx = (self.next + i) % self.peers.len();
            let (id, ref mut peer) = self.peers[idx];

            match peer.rx.poll_recv(cx) {
                Poll::Ready(Some(delivery)) => {
                    self.next = idx + 1;

                    return Poll::Ready(Ok(match delivery {
                        Delivery::Message(message) => Envelope {
                            info: NO_INFO.clone(),
                            route: id,
                            message,
                        },

                        Delivery::Envelope(envelope) => envelope,
                    }));
                }

                Poll::Ready(None) => {
                    panic!("session was dropped");
                }

                _ => {}
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
    use crate::sync::Arc;
    use crate::zmtp;
    use claim::*;
    use futures::{Future, FutureExt};
    use std::net::IpAddr;

    const OPTIONS: Options = Options {
        outgoing_queue_size: 2,
        incoming_queue_size: 2,
        max_reconnect_interval: std::time::Duration::from_micros(1),
        heartbeat_timeout: std::time::Duration::from_micros(1),
    };

    #[test]
    fn recv_queues_messages_fairly() {
        let mut receiver = FairReceiver::default();
        let (peer1, mut pipe1) = Peer::create(&OPTIONS);
        let (peer2, mut pipe2) = Peer::create(&OPTIONS);
        receiver.insert(peer1.id, peer1.rx);
        receiver.insert(peer2.id, peer2.rx);

        assert_eq!(receiver.poll_recv(cx!()), Poll::Pending);

        pipe1.tx.try_send(delivery!(1)).unwrap();
        pipe1.tx.try_send(delivery!(2)).unwrap();

        pipe2.tx.try_send(delivery!(3)).unwrap();
        pipe2.tx.try_send(delivery!(4)).unwrap();

        assert_ready_eq!(receiver.poll_recv(cx!()), Ok(envelope!(1, pipe1.id)));
        assert_ready_eq!(receiver.poll_recv(cx!()), Ok(envelope!(3, pipe2.id)));
        assert_ready_eq!(receiver.poll_recv(cx!()), Ok(envelope!(2, pipe1.id)));
        assert_ready_eq!(receiver.poll_recv(cx!()), Ok(envelope!(4, pipe2.id)));

        receiver.remove(peer2.id);

        pipe1.tx.try_send(delivery!(5)).unwrap();
        pipe1.tx.try_send(delivery!(6)).unwrap();

        assert_ready_eq!(receiver.poll_recv(cx!()), Ok(envelope!(5, pipe1.id)));
        assert_ready_eq!(receiver.poll_recv(cx!()), Ok(envelope!(6, pipe1.id)));
    }

    #[test]
    #[should_panic(expected = "session was dropped")]
    fn recv_panics_if_session_queue_is_dropped() {
        let mut receiver = FairReceiver::default();
        let (peer, pipe) = Peer::create(&OPTIONS);
        let id = pipe.id;
        drop(pipe);
        receiver.insert(peer.id, peer.rx);
        let _ = receiver.poll_recv(cx!());
    }
}
