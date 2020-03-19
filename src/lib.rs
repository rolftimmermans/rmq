#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]
#![allow(dead_code)] // todo
#![allow(unused_imports)] // todo
#![allow(unused_variables)] // todo

#[macro_use]
pub mod macros;

#[cfg(test)]
#[macro_use]
mod test;

mod dispatch;
mod endpoint;
mod error;
mod message;
mod session;
mod socket;
mod sync;
mod util;
mod zmtp;

pub use endpoint::{Endpoint, ToEndpoint};
pub use error::Error;
pub use message::{Envelope, Group, IntoMessage, Message, Route};

use futures::future::poll_fn;
use futures::{Future, FutureExt};
use std::pin::Pin;

macro_rules! define_socket {
    ($name:ident) => {
        #[derive(Default, Debug)]
        pub struct $name {
            inner: socket::Socket<socket::$name>,
        }

        impl $name {
            pub fn with_options(options: socket::Options) -> Self {
                Self {
                    inner: socket::Socket::with_options(options),
                }
            }

            pub async fn listen(&self, addr: impl ToEndpoint) -> Result<Endpoint, Error> {
                self.inner.listen(addr).await
            }

            pub async fn connect(&self, addr: impl ToEndpoint) -> Result<Route, Error> {
                self.inner.connect(addr).await
            }
        }
    };
}

macro_rules! define_recv {
    ($name:ident) => {
        impl $name {
            pub async fn recv(&self) -> Result<Envelope<Message>, Error> {
                poll_fn(|cx| self.inner.rx().poll_recv(cx)).await
            }
        }
    };
}

macro_rules! define_send {
    ($name:ident) => {
        impl $name {
            pub async fn send(&self, message: impl IntoMessage) -> Result<(), Error> {
                let mut message = Some(message.into_message());
                poll_fn(|cx| self.inner.tx().poll_send(&mut message, cx)).await
            }
        }
    };
}

macro_rules! define_route {
    ($name:ident) => {
        impl $name {
            pub async fn route(
                &self,
                message: impl IntoMessage,
                route: Route,
            ) -> Result<(), Error> {
                let mut message = Some(message.into_message());
                poll_fn(|cx| self.inner.tx().poll_route(&mut message, route, cx)).await
            }
        }
    };
}

define_socket!(Server);
define_recv!(Server);
define_route!(Server);

define_socket!(Client);
define_recv!(Client);
define_send!(Client);

define_socket!(Scatter);
define_send!(Scatter);

define_socket!(Gather);
define_recv!(Gather);

define_socket!(Radio);
impl Radio {
    pub fn broadcast(&self, message: impl IntoMessage, group: Group) -> Result<(), Error> {
        self.inner.tx().publish(message.into_message());
        Ok(())
    }
}

define_socket!(Dish);
define_recv!(Dish);
impl Dish {
    pub fn join(&self, group: Group) {
        self.inner.base().join(group)
    }

    pub fn leave(&self, group: Group) {
        self.inner.base().leave(group)
    }
}

define_socket!(Peer);
define_recv!(Peer);
define_route!(Peer);
