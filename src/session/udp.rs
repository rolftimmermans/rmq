use futures::stream::Stream;
use std::collections::HashSet;
use std::convert::TryInto;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net;
use tokio::stream::StreamExt;
use tokio::sync::mpsc;

use super::{Engine, Pipe, Session};

use crate::{
    dispatch::{Delivery, Registry},
    message::{Envelope, Info, Payload},
    sync::Arc,
    zmtp, Endpoint, Group, Message, Route,
};

impl Engine {
    pub(crate) async fn udp_listen(
        self,
        addr: std::net::SocketAddr,
    ) -> Result<Endpoint, crate::Error> {
        let socket = net::UdpSocket::bind(addr).await?;
        let addr = socket.local_addr()?;
        let pipe = self.peers.create(&self.options);
        tokio::spawn(self.udp_connect_internal(socket, pipe));
        Ok(Endpoint::Udp(addr))
    }

    pub(crate) async fn udp_connect(
        self,
        addr: SocketAddr,
        pipe: Pipe,
    ) -> Result<(), crate::Error> {
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
        let socket = net::UdpSocket::bind(bind_addr).await?;
        socket.connect(addr).await?;
        tokio::spawn(self.udp_connect_internal(socket, pipe));
        Ok(())
    }

    // async fn udp_listen_internal(mut self, mut listener: net::UdpListener) {
    //     let mut incoming = listener.incoming();
    //     while let Some(transport) = incoming.next().await {
    //         let transport = transport.expect("accept");
    //         let address = transport.peer_addr().ok();
    //         let session = Session::establish(&mut self, transport, None, address).await;
    //         tokio::spawn(session.run());
    //     }
    // }

    async fn udp_connect_internal(self, transport: net::UdpSocket, pipe: Pipe) {
        let socket = Socket {
            transport: zmtp::udp::frame(transport),
            pipe,
            next_frame: None,
        };
    }
}

#[derive(Debug)]
pub(crate) enum Error {
    QueueClosed,
    TransportClosed,
    // UnexpectedMultipartMessage,
    // UnexpectedFrame,
    InvalidGroup,
    // MissingGroup,
}

#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub(crate) struct Socket {
    transport: zmtp::udp::Framed,
    pipe: Pipe,
    next_frame: Option<zmtp::udp::Frame>,
}

impl Socket {
    fn poll_incoming(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if let Err(_) = futures::ready!(self.pipe.tx.poll_ready(cx)) {
                return Poll::Ready(Err(Error::QueueClosed));
            }

            tokio::pin! {
                let reader = &mut self.transport;
            }

            match futures::ready!(reader.poll_next(cx)) {
                Some(Ok((zmtp::udp::Frame::Message { payload, group }, addr))) => {
                    trace!("session::udp", "receiving message; len={}", payload.len());

                    let group = group.as_ref().try_into().map_err(|_| Error::InvalidGroup)?;

                    // Only deliver if this socket is subscribed to the group.
                    if self.pipe.groups.rx.borrow().contains(&group) {
                        let delivery = Delivery::Envelope(Envelope {
                            info: Arc::new(Info {
                                peer_address: Some(addr),
                                ..Default::default()
                            }),
                            route: self.pipe.id,
                            message: Message {
                                payload: Payload::from(payload),
                                group,
                            },
                        });

                        if let Err(mpsc::error::TrySendError::Closed(..)) =
                            self.pipe.tx.try_send(delivery)
                        {
                            return Poll::Ready(Err(Error::QueueClosed));
                        }
                    }
                }

                Some(Err(..)) | None => {
                    return Poll::Ready(Err(Error::TransportClosed));
                }
            }
        }
    }
}

// impl<T: AsyncWrite + Unpin> Session<T> {
//     fn poll_deliver(
//         &mut self,
//         cx: &mut Context<'_>,
//         frame: zmtp::Frame,
//     ) -> Poll<Result<(), Error>> {
//         debug_assert!(self.next_frame.is_none());

//         tokio::pin! {
//             let writer = &mut self.transport;
//         }

//         match writer.poll_ready(cx) {
//             Poll::Pending => {
//                 trace!("buffered outgoing frame");
//                 self.next_frame.replace(frame);
//                 Poll::Pending
//             }

//             Poll::Ready(Ok(())) => {
//                 tokio::pin! {
//                     let writer = &mut self.transport;
//                 }

//                 Poll::Ready(writer.start_send(frame).map_err(|_| Error::TransportClosed))
//             }

//             Poll::Ready(Err(..)) => {
//                 return Poll::Ready(Err(Error::TransportClosed));
//             }
//         }
//     }

//     fn poll_outgoing(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
//         if let Some(frame) = self.next_frame.take() {
//             futures::ready!(self.poll_deliver(cx, frame)?);
//         }

//         debug_assert!(self.next_frame.is_none());

//         loop {
//             match self.pipe.rx.poll_recv(cx) {
//                 Poll::Pending => {
//                     tokio::pin! {
//                         let writer = &mut self.transport;
//                     }

//                     trace!("flushing transport");

//                     futures::ready!(writer.poll_flush(cx)).map_err(|_| Error::TransportClosed)?;
//                     return Poll::Pending;
//                 }

//                 Poll::Ready(Some(Delivery::Message(message)))
//                 | Poll::Ready(Some(Delivery::Envelope(Envelope { message, .. }))) => {
//                     trace!("sending message; len={}", message.payload.len());

//                     futures::ready!(self.poll_deliver(
//                         cx,
//                         zmtp::Frame::Message {
//                             more: false,
//                             payload: message.payload.into_bytes(),
//                         }
//                     ))?;
//                 }

//                 Poll::Ready(None) => {
//                     return Poll::Ready(Err(Error::QueueClosed));
//                 }
//             }
//         }
//     }
// }
