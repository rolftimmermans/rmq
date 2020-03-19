use std::ops::DerefMut;
use tokio::sync::mpsc;

use super::{Delivery, Register, Sender};
use crate::sync::Mutex;
use crate::{Error, Group, Message, Route};

#[derive(Debug, Default)]
pub(crate) struct Publisher {
    peers: Vec<(Route, Sender)>,
}

impl Register<Sender> for Publisher {
    fn insert(&mut self, id: Route, peer: Sender) {
        self.peers.push((id, peer));
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

impl Publisher {
    pub(crate) fn publish(&mut self, message: Message) {
        for (_, peer) in self.peers.iter_mut() {
            let message = Delivery::Message(message.clone());
            if let Err(mpsc::error::TrySendError::Closed(..)) = peer.tx.try_send(message) {
                panic!("session was dropped");
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::dispatch::Peer;
//     use crate::socket::Options;
//     use claim::*;
//     use futures::{Future, FutureExt};
//     use std::collections::HashSet;

//     const OPTIONS: Options = Options {
//         outgoing_queue_size: 2,
//         incoming_queue_size: 2,
//         max_reconnect_interval: std::time::Duration::from_micros(1),
//         heartbeat_timeout: std::time::Duration::from_micros(1),
//     };

//     #[test]
//     fn broadcast_queues_messages_by_group() {
//         let mut publisher = Publisher::default();
//         let (peer1, mut pipe1) = Peer::create(&OPTIONS);
//         let (peer2, mut pipe2) = Peer::create(&OPTIONS);
//         publisher.insert(peer1.id, peer1.tx);
//         publisher.insert(peer2.id, peer2.tx);

//         let group1: Group = "A".parse().unwrap();
//         let group2: Group = "B".parse().unwrap();

//         pipe1.groups.tx.broadcast(set!(group1)).unwrap();
//         pipe2.groups.tx.broadcast(set!(group1, group2)).unwrap();

//         publisher.publish(msg!(1, group: group1));
//         publisher.publish(msg!(2, group: group2));
//         publisher.publish(msg!(3, group: group1));
//         publisher.publish(msg!(4, group: group2));
//         publisher.publish(msg!(5, group: group1));

//         assert_ok_eq!(pipe1.rx.try_recv(), delivery!(1, group: group1));
//         assert_ok_eq!(pipe1.rx.try_recv(), delivery!(3, group: group1));
//         assert_err!(pipe1.rx.try_recv());

//         assert_ok_eq!(pipe2.rx.try_recv(), delivery!(1, group: group1));
//         assert_ok_eq!(pipe2.rx.try_recv(), delivery!(2, group: group2));
//         assert_err!(pipe2.rx.try_recv());

//         publisher.remove(peer2.id);

//         publisher.publish(msg!(5, group: group1));
//         publisher.publish(msg!(6, group: group2));

//         assert_ok_eq!(pipe1.rx.try_recv(), delivery!(5, group: group1));
//         assert_err!(pipe1.rx.try_recv());
//     }

//     #[test]
//     #[should_panic(expected = "session was dropped")]
//     fn broadcast_panics_if_session_queue_is_dropped() {
//         let mut router = Publisher::default();
//         let (peer, pipe) = Peer::create(&OPTIONS);

//         let group: Group = "A".parse().unwrap();
//         pipe.groups.tx.broadcast(set!(group)).unwrap();

//         let id = pipe.id;
//         drop(pipe);

//         router.insert(peer.id, peer.tx);
//         let _ = router.publish(msg!(1, group: group));
//     }
// }
