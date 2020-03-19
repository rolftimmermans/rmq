use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tokio::sync::{broadcast, mpsc, watch};

use crate::message::Info;
use crate::socket::Options;
use crate::sync::Arc;
use crate::util;
use crate::{Envelope, Error, Group, Message, Route};

lazy_static::lazy_static! {
    static ref SEQUENCE: util::Sequence = util::Sequence::default();
}

#[derive(Debug)]
pub(crate) struct Exchange<T> {
    pub(crate) tx: watch::Sender<T>,
    pub(crate) rx: watch::Receiver<T>,
}

impl<T: Default + Clone> Default for Exchange<T> {
    fn default() -> Self {
        let (tx, rx) = watch::channel(Default::default());
        Self { tx, rx }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Delivery {
    Message(Message),
    Envelope(Envelope<Message>),
}

#[derive(Debug)]
pub(crate) struct Sender {
    pub(crate) tx: mpsc::Sender<Delivery>,
    pub(crate) groups: Arc<Exchange<HashSet<Group>>>,
}

#[derive(Debug)]
pub(crate) struct Receiver {
    pub(crate) rx: mpsc::Receiver<Delivery>,
    pub(crate) groups: Arc<Exchange<HashSet<Group>>>,
}

#[derive(Debug)]
pub(crate) struct Peer {
    pub(crate) id: Route,
    pub(crate) tx: Sender,
    pub(crate) rx: Receiver,
}

#[derive(Debug)]
pub(crate) struct Pipe {
    pub(crate) id: Route,
    pub(crate) tx: mpsc::Sender<Delivery>,
    pub(crate) rx: mpsc::Receiver<Delivery>,

    pub(crate) groups: Arc<Exchange<HashSet<Group>>>,
}

impl Peer {
    pub(super) fn create(options: &Options) -> (Peer, Pipe) {
        let id = SEQUENCE.next();

        let (outgoing_tx, outgoing_rx) = mpsc::channel(options.outgoing_queue_size);
        let (incoming_tx, incoming_rx) = mpsc::channel(options.incoming_queue_size);

        let groups: Arc<Exchange<HashSet<Group>>> = Default::default();

        let peer = Peer {
            id: Route { id },
            tx: Sender {
                tx: outgoing_tx,
                groups: groups.clone(),
            },
            rx: Receiver {
                rx: incoming_rx,
                groups: groups.clone(),
            },
        };

        let pipe = Pipe {
            id: Route { id },
            tx: incoming_tx,
            rx: outgoing_rx,
            groups: groups.clone(),
        };

        (peer, pipe)
    }

    pub(super) fn attach(pipe: Pipe) -> Peer {
        let id = SEQUENCE.next();

        Peer {
            id: Route { id },
            tx: Sender {
                tx: pipe.tx,
                groups: pipe.groups.clone(),
            },
            rx: Receiver {
                rx: pipe.rx,
                groups: pipe.groups,
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
