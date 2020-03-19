use std::collections::{HashMap, HashSet};
use tokio::sync::watch;

mod peer;

mod fair_receiver;
mod fair_sender;
mod publisher;
mod router;

pub(super) use fair_receiver::FairReceiver;
pub(super) use fair_sender::FairSender;
pub(super) use publisher::Publisher;
pub(super) use router::Router;

pub(crate) use peer::{Delivery, Peer, Pipe, Receiver, Sender};

use crate::message::{Info, Route, Group};
use crate::socket::Options;
use crate::sync::{Arc, Mutex, MutexGuard};

pub(crate) trait Register<T>: std::fmt::Debug + Send + Sync + 'static {
    fn insert(&mut self, id: Route, item: T);
    fn remove(&mut self, id: Route);
}

impl<T> Register<T> for () {
    fn insert(&mut self, id: Route, peer: T) {}
    fn remove(&mut self, id: Route) {}
}

#[derive(Debug, Clone)]
pub(crate) struct Registry {
    tx: Arc<Mutex<dyn Register<Sender>>>,
    rx: Arc<Mutex<dyn Register<Receiver>>>,
}

impl Registry {
    pub(crate) fn create(&self, options: &Options) -> Pipe {
        let (peer, pipe) = Peer::create(options);
        self.insert(peer);
        pipe
    }

    pub(crate) fn attach(&self, pipe: Pipe) {
        let peer = Peer::attach(pipe);
        self.insert(peer);
    }

    pub(crate) fn remove(&self, id: Route) {
        self.tx.lock().remove(id);
        self.rx.lock().remove(id);
        debug!("dispatch", "peer removed; id={}", id);
    }

    fn insert(&self, peer: Peer) {
        self.tx.lock().insert(peer.id, peer.tx);
        self.rx.lock().insert(peer.id, peer.rx);
        debug!("dispatch", "peer inserted; id={}", peer.id);
    }
}

#[derive(Default)]
pub(crate) struct Dispatcher<S, R> {
    pub(crate) tx: Arc<Mutex<S>>,
    pub(crate) rx: Arc<Mutex<R>>,
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

impl<S, R> Dispatcher<S, R>
where
    S: Register<Sender>,
    R: Register<Receiver>,
{
    pub(crate) fn registry(&self) -> Registry {
        Registry {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }

    pub(crate) fn tx(&self) -> MutexGuard<'_, S> {
        self.tx.lock()
    }

    pub(crate) fn rx(&self) -> MutexGuard<'_, R> {
        self.rx.lock()
    }
}
