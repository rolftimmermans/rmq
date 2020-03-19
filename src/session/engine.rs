use std::fmt;
use std::net;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::collections::HashSet;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

use crate::{
    dispatch::{Pipe, Registry, Exchange},
    message::Info,
    socket::Options,
    sync::Arc,
    zmtp::SocketType,
    Endpoint, Error, Message, Group, Route, ToEndpoint,
};

use super::Session;

pub(crate) struct Engine {
    pub(crate) socket_type: SocketType,
    pub(crate) remote_type: SocketType,
    pub(crate) peers: Registry,
    pub(crate) groups: Option<tokio::sync::watch::Receiver<Vec<Group>>>,
    pub(crate) options: Options,
}

impl fmt::Debug for Engine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Engine")
            .field("socket_type", &self.socket_type)
            .field("remote_type", &self.remote_type)
            .finish() // TODO: finish_non_exhaustive()
    }
}

impl Engine {
    pub(crate) async fn listen(self, addr: impl ToEndpoint) -> Result<Endpoint, Error> {
        let addr = addr.to_endpoint().await?;
        debug!("engine", "starting listener; addr={}", addr);

        match addr {
            #[cfg(feature = "tcp")]
            Endpoint::Tcp(addr) => self.tcp_listen(addr).await,

            #[cfg(feature = "udp")]
            Endpoint::Udp(addr) => self.udp_listen(addr).await,

            #[cfg(feature = "ipc")]
            Endpoint::Ipc(addr) => self.ipc_listen(addr).await,

            #[cfg(feature = "inproc")]
            Endpoint::Inproc(addr) => self.inproc_listen(addr).await,
        }
    }

    pub(crate) async fn connect(self, addr: impl ToEndpoint) -> Result<Route, Error> {
        let addr = addr.to_endpoint().await?;
        debug!("engine", "starting connector; addr={}", addr);

        let pipe = self.create_pipe();
        let id = pipe.id;

        match addr {
            #[cfg(feature = "tcp")]
            Endpoint::Tcp(addr) => self.tcp_connect(addr, pipe).await?,

            #[cfg(feature = "udp")]
            Endpoint::Udp(addr) => self.udp_connect(addr, pipe).await?,

            #[cfg(feature = "ipc")]
            Endpoint::Ipc(addr) => self.ipc_connect(addr, pipe).await?,

            #[cfg(feature = "inproc")]
            Endpoint::Inproc(addr) => self.inproc_connect(addr, pipe).await?,
        };

        Ok(id)
    }

    pub(super) fn create_pipe(&self) -> Pipe {
        self.peers.create(&self.options)
    }
}
