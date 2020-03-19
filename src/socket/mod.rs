use std::fmt;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::dispatch::{Dispatcher, Receiver, Register, Sender};
use crate::message::Info;
use crate::session::Engine;
use crate::sync::{Arc, MutexGuard};
use crate::zmtp::SocketType;
use crate::util::Exchange;
use crate::{Endpoint, Error, Group, Message, Route, ToEndpoint};

mod types;

pub(super) use types::*;

pub(super) trait Base: Send + Default {
    const SELF: SocketType;
    const PEER: SocketType;

    type Sender: Register<Sender> + Default;
    type Receiver: Register<Receiver> + Default;

    fn groups(&self) -> Option<tokio::sync::watch::Receiver<Vec<Group>>> {
        None
    }
}

#[derive(Default)]
pub(crate) struct Socket<T: Base> {
    base: T,
    dispatcher: Dispatcher<T::Sender, T::Receiver>,
    options: Options,
}

#[derive(Debug, Copy, Clone)]
pub struct Options {
    pub outgoing_queue_size: usize,
    pub incoming_queue_size: usize,
    pub heartbeat_timeout: Duration,
    pub max_reconnect_interval: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            outgoing_queue_size: 1024,
            incoming_queue_size: 1024,
            heartbeat_timeout: Duration::from_secs(10),
            max_reconnect_interval: Duration::from_secs(30),
        }
    }
}

impl<T: Base> fmt::Debug for Socket<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Socket")
            .field("type", &T::SELF)
            .field("options", &self.options)
            .finish() // TODO: finish_non_exhaustive()
    }
}

impl<T: Base> Socket<T> {
    pub(super) fn with_options(options: Options) -> Self {
        Self {
            options,
            ..Default::default()
        }
    }

    pub(super) async fn listen<'a>(&self, addr: impl ToEndpoint) -> Result<Endpoint, Error> {
        self.create_engine().listen(addr).await
    }

    pub(super) async fn connect(&self, addr: impl ToEndpoint) -> Result<Route, Error> {
        self.create_engine().connect(addr).await
    }

    pub(super) fn base(&self) -> &T {
        &self.base
    }

    pub(super) fn tx(&self) -> MutexGuard<'_, T::Sender> {
        self.dispatcher.tx()
    }

    pub(super) fn rx(&self) -> MutexGuard<'_, T::Receiver> {
        self.dispatcher.rx()
    }

    fn create_engine(&self) -> Engine {
        Engine {
            socket_type: T::SELF,
            remote_type: T::PEER,
            peers: self.dispatcher.registry(),
            groups: self.base.groups(),
            options: self.options,
        }
    }
}
