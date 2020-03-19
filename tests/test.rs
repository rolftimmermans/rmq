#![allow(dead_code)]
use std::sync::atomic::{AtomicU32, Ordering};

pub static MSG: &str = "12345678ABCDEFGH12345678abcdefgh";

lazy_static::lazy_static! {
    pub static ref TMPDIR: tempfile::TempDir = tempfile::tempdir().unwrap();
}

include!("./macros.rs");

#[derive(Debug, Copy, Clone)]
pub enum Transport {
    TCP,
    IPC,
    INPROC,
}

pub fn transports() -> Vec<Transport> {
    vec![
        #[cfg(feature = "tcp")]
        Transport::TCP,
        #[cfg(feature = "ipc")]
        Transport::IPC,
        #[cfg(feature = "inproc")]
        Transport::INPROC,
    ]
}

pub static PORT: AtomicU32 = AtomicU32::new(40000);

pub fn endpoint(transport: Transport) -> String {
    match transport {
        Transport::TCP => {
            let port = PORT.fetch_add(1, Ordering::SeqCst);
            format!("tcp://127.0.0.1:{}", port)
        }

        Transport::IPC => {
            let id: usize = rand::random();
            format!(
                "ipc://{}",
                TMPDIR
                    .path()
                    .join(format!("rmq-test-{}", id))
                    .to_str()
                    .unwrap()
            )
        }

        Transport::INPROC => {
            let id: usize = rand::random();
            format!("inproc://rmq-test-{}", id)
        }
    }
}
