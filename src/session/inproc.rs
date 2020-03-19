use std::collections::HashMap;

use super::{Engine, Info, Pipe, Session};
use crate::sync::{Arc, RwLock};
use crate::{Endpoint, Error, Route};

lazy_static::lazy_static! {
    static ref ENDPOINTS: RwLock<HashMap<String, Engine>> = Default::default();
}

impl Engine {
    pub(crate) async fn inproc_listen(self, addr: String) -> Result<Endpoint, Error> {
        ENDPOINTS.write().insert(addr.clone(), self);
        Ok(Endpoint::Inproc(addr))
    }

    pub(crate) async fn inproc_connect<'a>(self, addr: String, pipe: Pipe) -> Result<(), Error> {
        if let Some(engine) = ENDPOINTS.read().get(&addr) {
            engine.peers.attach(pipe);
            return Ok(());
        }

        tokio::spawn(async move {
            loop {
                tokio::time::delay_for(std::time::Duration::from_millis(10)).await;
                if let Some(engine) = ENDPOINTS.read().get(&addr) {
                    engine.peers.attach(pipe);
                    return;
                }
            }
        });

        Ok(())
    }
}
