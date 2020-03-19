use std::net::SocketAddr;
use tokio::net;
use tokio::stream::StreamExt;

use super::{Engine, Pipe, Session};
use crate::{Endpoint, Error, Route};

impl Engine {
    pub(crate) async fn tcp_listen(self, addr: std::net::SocketAddr) -> Result<Endpoint, Error> {
        let listener = net::TcpListener::bind(addr).await?;
        let addr = listener.local_addr()?;
        tokio::spawn(self.tcp_listen_internal(listener));
        Ok(Endpoint::Tcp(addr))
    }

    pub(crate) async fn tcp_connect(self, addr: SocketAddr, pipe: Pipe) -> Result<(), Error> {
        tokio::spawn(self.tcp_connect_internal(addr, pipe));
        Ok(())
    }

    async fn tcp_listen_internal(mut self, mut listener: net::TcpListener) {
        let mut incoming = listener.incoming();
        while let Some(transport) = incoming.next().await {
            let transport = transport.expect("accept");
            let address = transport.peer_addr().ok();
            let session = Session::establish(&mut self, transport, None, address).await;
            tokio::spawn(session.run());
        }
    }

    async fn tcp_connect_internal(mut self, addr: SocketAddr, mut pipe: Pipe) {
        loop {
            match net::TcpStream::connect(addr).await {
                Ok(transport) => {
                    let address = transport.peer_addr().ok();
                    let session =
                        Session::establish(&mut self, transport, Some(pipe), address).await;
                    pipe = session.run().await;
                    return;
                }
                Err(_err) => {
                    tokio::time::delay_for(std::time::Duration::from_millis(10)).await;
                }
            }
        }
    }
}
