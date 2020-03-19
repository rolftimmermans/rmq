use futures::{future, Future, FutureExt};
use std::{fmt, net, pin};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Endpoint {
    #[cfg(feature = "tcp")]
    Tcp(net::SocketAddr),

    #[cfg(feature = "udp")]
    Udp(net::SocketAddr),

    #[cfg(feature = "ipc")]
    Ipc(String),

    #[cfg(feature = "inproc")]
    Inproc(String),
}

type EndpointFuture = pin::Pin<Box<dyn Future<Output = Result<Endpoint, Error>> + Send + 'static>>;

pub trait ToEndpoint {
    fn to_endpoint(self) -> EndpointFuture;
}

impl ToEndpoint for Endpoint {
    fn to_endpoint(self) -> EndpointFuture {
        future::ok(self).boxed()
    }
}

impl ToEndpoint for &Endpoint {
    fn to_endpoint(self) -> EndpointFuture {
        self.clone().to_endpoint()
    }
}

impl<T: AsRef<str>> ToEndpoint for T {
    fn to_endpoint(self) -> EndpointFuture {
        let mut split = self.as_ref().splitn(2, "://");
        match (split.next(), split.next().map(str::to_owned)) {
            (Some("tcp"), Some(addr)) => {
                #[cfg(feature = "tcp")]
                {
                    async { Ok(Endpoint::Tcp(resolve(addr).await?)) }.boxed()
                }

                #[cfg(not(feature = "tcp"))]
                {
                    future::err(Error::TransportUnavailable).boxed()
                }
            }

            (Some("udp"), Some(addr)) => {
                #[cfg(feature = "udp")]
                {
                    async { Ok(Endpoint::Udp(resolve(addr).await?)) }.boxed()
                }

                #[cfg(not(feature = "udp"))]
                {
                    future::err(Error::TransportUnavailable).boxed()
                }
            }

            (Some("ipc"), Some(addr)) => {
                #[cfg(feature = "ipc")]
                {
                    future::ok(Endpoint::Ipc(addr)).boxed()
                }

                #[cfg(not(feature = "ipc"))]
                {
                    future::err(Error::TransportUnavailable).boxed()
                }
            }

            (Some("inproc"), Some(addr)) => {
                #[cfg(feature = "inproc")]
                {
                    future::ok(Endpoint::Inproc(addr)).boxed()
                }

                #[cfg(not(feature = "inproc"))]
                {
                    future::err(Error::TransportUnavailable).boxed()
                }
            }

            (Some(_), Some(_)) => future::err(Error::TransportUnknown).boxed(),
            _ => future::err(Error::AddressInvalid).boxed(),
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "tcp")]
            Endpoint::Tcp(addr) => write!(f, "tcp://{}", addr),

            #[cfg(feature = "udp")]
            Endpoint::Udp(addr) => write!(f, "udp://{}", addr),

            #[cfg(feature = "ipc")]
            Endpoint::Ipc(addr) => write!(f, "ipc://{}", addr),

            #[cfg(feature = "inproc")]
            Endpoint::Inproc(addr) => write!(f, "inproc://{}", addr),
        }
    }
}

async fn resolve(addr: String) -> Result<net::SocketAddr, Error> {
    match tokio::net::lookup_host(addr).await {
        Ok(mut res) => res.next().ok_or(Error::AddressNotFound),
        Err(err) if err.kind() == std::io::ErrorKind::InvalidInput => Err(Error::AddressInvalid),
        Err(_) => Err(Error::AddressNotFound),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "tcp")]
    #[tokio::test]
    async fn to_endpoint_returns_resolved_tcp() {
        let resolved = "tcp://localhost:1234".to_endpoint().await.unwrap();
        assert!(
            resolved == Endpoint::Tcp("127.0.0.1:1234".parse().unwrap())
                || resolved == Endpoint::Tcp("[::1]:1234".parse().unwrap())
        );
    }

    #[cfg(feature = "udp")]
    #[tokio::test]
    async fn to_endpoint_returns_resolved_udp() {
        let resolved = "udp://localhost:1234".to_endpoint().await.unwrap();
        assert!(
            resolved == Endpoint::Udp("127.0.0.1:1234".parse().unwrap())
                || resolved == Endpoint::Udp("[::1]:1234".parse().unwrap())
        );
    }

    #[cfg(feature = "ipc")]
    #[tokio::test]
    async fn to_endpoint_returns_ipc() {
        let resolved = "ipc:///var/tmp/sock".to_endpoint().await.unwrap();
        assert_eq!(resolved, Endpoint::Ipc("/var/tmp/sock".to_owned()));
    }

    #[cfg(feature = "inproc")]
    #[tokio::test]
    async fn to_endpoint_returns_inproc() {
        let resolved = "inproc://my-endpoint".to_endpoint().await.unwrap();
        assert_eq!(resolved, Endpoint::Inproc("my-endpoint".to_owned()));
    }

    #[tokio::test]
    async fn to_endpoint_returns_error() {
        assert_eq!(
            Err(Error::TransportUnknown),
            "foobar://my-endpoint".to_endpoint().await
        );
        assert_eq!(
            Err(Error::AddressInvalid),
            "foo bar baz".to_endpoint().await
        );
    }
}
