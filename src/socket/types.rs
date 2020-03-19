use crate::dispatch::{FairReceiver, FairSender, Publisher, Router};
use crate::Group;
use crate::sync::Mutex;
use crate::util::Exchange;

use super::{Base, SocketType};

#[derive(Debug, Default)]
pub(crate) struct Client;

impl Base for Client {
    const SELF: SocketType = SocketType::CLIENT;
    const PEER: SocketType = SocketType::SERVER;

    type Sender = FairSender;
    type Receiver = FairReceiver;
}

#[derive(Debug, Default)]
pub(crate) struct Server;

impl Base for Server {
    const SELF: SocketType = SocketType::SERVER;
    const PEER: SocketType = SocketType::CLIENT;

    type Sender = Router;
    type Receiver = FairReceiver;
}

#[derive(Debug, Default)]
pub(crate) struct Radio;

impl Base for Radio {
    const SELF: SocketType = SocketType::RADIO;
    const PEER: SocketType = SocketType::DISH;

    type Sender = Publisher;
    type Receiver = ();
}

#[derive(Debug, Default)]
pub(crate) struct Dish {
    groups: Mutex<Vec<Group>>,
    exchange: Exchange<Vec<Group>>,
}

impl Dish {
    pub(crate) fn join(&self, group: Group) {
        let mut groups = self.groups.lock();

        if !groups.contains(&group) {
            groups.push(group);
        }

        let _ = self.exchange.tx.broadcast(groups.clone());
    }

    pub(crate) fn leave(&self, group: Group) {
        let mut groups = self.groups.lock();

        let mut i = 0;
        while i != groups.len() {
            if groups[i] == group {
                groups.swap_remove(i);
            } else {
                i += 1;
            }
        }

        let _ = self.exchange.tx.broadcast(groups.clone());
    }
}

impl Base for Dish {
    const SELF: SocketType = SocketType::DISH;
    const PEER: SocketType = SocketType::RADIO;

    type Sender = ();
    type Receiver = FairReceiver;

    fn groups(&self) -> Option<tokio::sync::watch::Receiver<Vec<Group>>> {
        Some(self.exchange.rx.clone())
    }
}

#[derive(Debug, Default)]
pub(crate) struct Scatter;

impl Base for Scatter {
    const SELF: SocketType = SocketType::SCATTER;
    const PEER: SocketType = SocketType::GATHER;

    type Sender = FairSender;
    type Receiver = ();
}

#[derive(Debug, Default)]
pub(crate) struct Gather;

impl Base for Gather {
    const SELF: SocketType = SocketType::GATHER;
    const PEER: SocketType = SocketType::SCATTER;

    type Sender = ();
    type Receiver = FairReceiver;
}

#[derive(Debug, Default)]
pub(crate) struct Peer;

impl Base for Peer {
    const SELF: SocketType = SocketType::PEER;
    const PEER: SocketType = SocketType::PEER;

    type Sender = Router;
    type Receiver = FairReceiver;
}
