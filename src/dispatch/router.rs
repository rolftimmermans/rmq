use futures::future::poll_fn;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::task::{Context, Poll};

use super::{Delivery, Register, Sender};
use crate::sync::{Arc, Mutex};
use crate::{Error, Message, Route};

#[derive(Debug, Default)]
pub(crate) struct Router {
    peers: HashMap<Route, Sender>,
}

impl Register<Sender> for Router {
    fn insert(&mut self, id: Route, peer: Sender) {
        self.peers.insert(id, peer);
    }

    fn remove(&mut self, id: Route) {
        if self.peers.remove(&id).is_none() {
            panic!("removing unknown peer");
        }
    }
}

impl Router {
    pub(crate) fn poll_route(
        &mut self,
        message: &mut Option<Message>,
        id: Route,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), Error>> {
        let peer = match self.peers.get_mut(&id) {
            Some(tx) => tx,
            None => return Poll::Ready(Err(Error::RoutingError)),
        };

        if let Err(..) = futures::ready!(peer.tx.poll_ready(cx)) {
            panic!("session was dropped")
        }

        let message = message.take().expect("message taken before send");
        if let Err(..) = peer.tx.try_send(Delivery::Message(message)) {
            panic!("session was dropped")
        }

        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::Peer;
    use crate::socket::Options;
    use claim::*;
    use futures::{Future, FutureExt};

    const OPTIONS: Options = Options {
        outgoing_queue_size: 1,
        incoming_queue_size: 1,
        max_reconnect_interval: std::time::Duration::from_micros(1),
        heartbeat_timeout: std::time::Duration::from_micros(1),
    };

    #[test]
    fn route_queues_messages_by_identity() {
        let mut router = Router::default();
        let (peer, mut pipe) = Peer::create(&OPTIONS);
        router.insert(peer.id, peer.tx);

        assert_ready_eq!(
            router.poll_route(&mut Some(msg!(1)), pipe.id, cx!()),
            Ok(())
        );

        assert_pending!(router.poll_route(&mut Some(msg!(1)), pipe.id, cx!()));
        assert_ok_eq!(pipe.rx.try_recv(), delivery!(1));

        assert_ready_eq!(
            router.poll_route(&mut Some(msg!(2)), pipe.id, cx!()),
            Ok(())
        );
        assert_ok_eq!(pipe.rx.try_recv(), delivery!(2));
    }

    #[test]
    fn route_returns_error_for_unknown_ids() {
        let mut router = Router::default();
        assert_ready_eq!(
            router.poll_route(&mut Some(msg!(1)), Route { id: 1 }, cx!()),
            Err(Error::RoutingError)
        );
    }

    #[test]
    #[should_panic(expected = "session was dropped")]
    fn route_panics_if_session_queue_is_dropped() {
        let mut router = Router::default();
        let (peer, pipe) = Peer::create(&OPTIONS);

        let id = pipe.id;
        drop(pipe);

        router.insert(peer.id, peer.tx);
        let _ = router.poll_route(&mut Some(msg!(1)), id, cx!());
    }
}
