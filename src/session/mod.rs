#![cfg(not(loom))]

use futures::{Future, FutureExt, Sink, SinkExt, Stream, StreamExt};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;

use crate::{
    dispatch::{Delivery, Exchange, Pipe, Registry},
    message::{Envelope, Info, Payload},
    zmtp, Group, Message,
};

use crate::sync::Arc;

mod engine;

#[cfg(feature = "inproc")]
mod inproc;

// #[cfg(feature = "ipc")]
// mod ipc;

#[cfg(feature = "tcp")]
mod tcp;

#[cfg(feature = "udp")]
mod udp;

pub(crate) use engine::Engine;

#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub(crate) struct Session<T> {
    transport: zmtp::Framed<T>,
    pipe: Pipe,
    info: Arc<Info>,
    groups: Option<tokio::sync::watch::Receiver<Vec<Group>>>,
    peer_groups: Vec<Group>,
    next_group: Option<Group>,
    next_frame: Option<zmtp::Frame>,
}

#[derive(Debug)]
pub(crate) enum Error {
    QueueClosed,
    InvalidPeerSocket,
    TransportClosed,
    UnexpectedMultipartMessage,
    UnexpectedFrame,
    InvalidGroup,
    MissingGroup,
}

impl<T: AsyncRead + AsyncWrite + Unpin> Session<T> {
    async fn establish(
        engine: &mut Engine,
        transport: T,
        pipe: Option<Pipe>,
        address: Option<SocketAddr>,
    ) -> Self {
        let params = zmtp::Params {
            socket_type: engine.socket_type,
            security: zmtp::Security::Null,
        };

        let mut transport = zmtp::frame(transport);
        let mut remote = zmtp::connect(&mut transport, &params).await.unwrap();

        if remote.socket_type != engine.remote_type {
            // Peer not of correct type
            return Err(Error::InvalidPeerSocket).unwrap();
        }

        debug!(
            "session",
            "established; peer={}",
            address
                .as_ref()
                .map(SocketAddr::to_string)
                .unwrap_or("unknown".to_owned())
        );

        Session {
            transport,
            pipe: match pipe {
                Some(pipe) => pipe,
                None => engine.peers.create(&engine.options),
            },
            info: Arc::new(Info {
                peer_address: address,
                identity: remote.properties.remove(zmtp::tag::IDENTITY),
                resource: remote.properties.remove(zmtp::tag::RESOURCE),
                custom: remote.properties,
            }),
            groups: engine.groups.clone(),
            peer_groups: Default::default(),
            next_group: None,
            next_frame: None,
        }
    }

    async fn run(mut self) -> Pipe {
        futures::future::poll_fn(|cx| self.poll(cx)).await;
        self.pipe
    }
}

impl<T: AsyncRead + Unpin> Session<T> {
    fn poll_incoming(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        loop {
            if let Err(_) = futures::ready!(self.pipe.tx.poll_ready(cx)) {
                return Poll::Ready(Err(Error::QueueClosed));
            }

            tokio::pin! {
                let reader = &mut self.transport;
            }

            match futures::ready!(reader.poll_next(cx)) {
                Some(Ok(zmtp::Frame::Message {
                    more: true,
                    payload,
                })) => {
                    if self.next_group.is_none() {
                        let group = payload
                            .as_ref()
                            .try_into()
                            .map_err(|_| Error::InvalidGroup)?;
                        self.next_group.replace(group);
                    } else {
                        return Poll::Ready(Err(Error::UnexpectedMultipartMessage));
                    }
                }

                Some(Ok(zmtp::Frame::Message {
                    more: false,
                    payload,
                })) => {
                    trace!("session", "receiving message; len={}", payload.len());

                    let delivery = Delivery::Envelope(Envelope {
                        info: self.info.clone(),
                        route: self.pipe.id,
                        message: Message {
                            payload: Payload::from(payload),
                            group: self.next_group.take().unwrap_or_default(),
                        },
                    });

                    if let Err(mpsc::error::TrySendError::Closed(..)) =
                        self.pipe.tx.try_send(delivery)
                    {
                        return Poll::Ready(Err(Error::QueueClosed));
                    }
                }

                Some(Ok(zmtp::Frame::Join { group })) => {
                    let group: Group = group.as_slice().try_into().expect("invalid group");
                    debug!("session", "peer joined; group={}", &group);
                    // self.peer_groups.push(group);
                    // self.groups
                    //     .tx
                    //     .broadcast(groups.clone())
                    //     .map_err(|_| Error::QueueClosed)?;
                }

                Some(Ok(zmtp::Frame::Leave { group })) => {
                    let group: Group = group.as_slice().try_into().expect("invalid group");
                    debug!("session", "peer left; group={}", &group);
                    // self.peer_groups.push(group);
                    // groups.remove(&group);
                    // self.pipe
                    //     .groups
                    //     .tx
                    //     .broadcast(groups.clone())
                    //     .map_err(|_| Error::QueueClosed)?;
                }

                Some(Ok(zmtp::Frame::Ping { ttl, context })) => {
                    // Reply with PONG unless we're already in the process of
                    // sending out a message to this peer.
                    if self.next_frame.is_none() {
                        trace!("session", "ping");
                        self.next_frame.replace(zmtp::Frame::Pong { context });
                    }
                }

                Some(Ok(..)) => {
                    return Poll::Ready(Err(Error::UnexpectedFrame));
                }

                Some(Err(..)) | None => {
                    return Poll::Ready(Err(Error::TransportClosed));
                }
            }
        }
    }
}

impl<T: AsyncWrite + Unpin> Session<T> {
    fn poll_deliver(
        &mut self,
        cx: &mut Context<'_>,
        frame: zmtp::Frame,
    ) -> Poll<Result<(), Error>> {
        debug_assert!(self.next_frame.is_none());

        tokio::pin! {
            let writer = &mut self.transport;
        }

        match writer.poll_ready(cx) {
            Poll::Pending => {
                trace!("session", "buffered outgoing frame");
                self.next_frame.replace(frame);
                Poll::Pending
            }

            Poll::Ready(Ok(())) => {
                tokio::pin! {
                    let writer = &mut self.transport;
                }

                Poll::Ready(writer.start_send(frame).map_err(|_| Error::TransportClosed))
            }

            Poll::Ready(Err(..)) => {
                return Poll::Ready(Err(Error::TransportClosed));
            }
        }
    }

    fn poll_outgoing(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        if let Some(frame) = self.next_frame.take() {
            futures::ready!(self.poll_deliver(cx, frame)?);
        }

        debug_assert!(self.next_frame.is_none());

        loop {
            if let Some(groups) = self.groups.as_mut() {
                match groups.poll_recv_ref(cx) {
                    Poll::Ready(Some(groups)) => {
                        // groups.
                        // TODO
                    }

                    _ => {}
                }
            }

            match self.pipe.rx.poll_recv(cx) {
                Poll::Pending => {
                    tokio::pin! {
                        let writer = &mut self.transport;
                    }

                    trace!("session", "flushing transport");

                    futures::ready!(writer.poll_flush(cx)).map_err(|_| Error::TransportClosed)?;
                    return Poll::Pending;
                }

                Poll::Ready(Some(Delivery::Message(message)))
                | Poll::Ready(Some(Delivery::Envelope(Envelope { message, .. }))) => {
                    trace!("session", "sending message; len={}", message.payload.len());

                    futures::ready!(self.poll_deliver(
                        cx,
                        zmtp::Frame::Message {
                            more: false,
                            payload: message.payload.into_bytes(),
                        }
                    ))?;
                }

                Poll::Ready(None) => {
                    return Poll::Ready(Err(Error::QueueClosed));
                }
            }
        }
    }
}

impl<T: AsyncRead + AsyncWrite + Unpin> Session<T> {
    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let mut ready = false;

        match self.poll_outgoing(cx) {
            Poll::Ready(Ok(())) => {
                debug!("session", "outgoing session ended");
            }

            Poll::Ready(Err(err)) => {
                debug!("session", "outgoing session error; err={:?}", err);
            }

            Poll::Pending => {
                ready = false;
            }
        }

        match self.poll_incoming(cx) {
            Poll::Ready(Ok(())) => {
                debug!("session", "incoming session ended");
            }

            Poll::Ready(Err(err)) => {
                debug!("session", "incoming session error; err={:?}", err);
            }

            Poll::Pending => {
                ready = false;
            }
        }

        if ready {
            return Poll::Ready(());
        } else {
            return Poll::Pending;
        }
    }
}

// impl<T: AsyncRead + AsyncWrite + Unpin> Future for Session<T> {
//     type Output = Pipe;

//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         futures::ready!(Session::poll(&mut self, cx));
//         Poll::Ready((*self).pipe)
//     }
// }
