use libzmq::prelude::{BuildSocket, RecvMsg, SendMsg, Socket, TryInto};
use rmq::{Client, Server};

mod test;
use claim::*;
use std::thread;

#[tokio::test]
async fn client_compat() {
    subscribe_tracing!();

    let address: libzmq::TcpAddr = "127.0.0.1:*".try_into().unwrap();
    let endpoint: libzmq::addr::Endpoint = address.into();

    let server = libzmq::ServerBuilder::new().bind(endpoint).build().unwrap();

    let addr = match server.last_endpoint().unwrap() {
        libzmq::addr::Endpoint::Tcp(addr) => format!("tcp://{}", addr),
        _ => panic!("unexpected proto"),
    };

    thread::spawn(move || {
        let msg1 = server.recv_msg().unwrap();
        assert_eq!(msg1.as_bytes(), b"hello 1");

        let msg2 = server.recv_msg().unwrap();
        assert_eq!(msg2.as_bytes(), b"hello 2");

        server
            .route("hello world 1", msg1.routing_id().unwrap())
            .unwrap();
        server
            .route("hello world 2", msg2.routing_id().unwrap())
            .unwrap();
    });

    let client1 = Client::default();
    assert_ok!(client1.connect(&addr).await);

    let client2 = Client::default();
    assert_ok!(client2.connect(&addr).await);

    tokio::time::delay_for(std::time::Duration::from_millis(1)).await;

    assert_ok!(client1.send("hello 1").await);
    assert_ok!(client2.send("hello 2").await);

    assert_ok_eq!(client1.recv().await, b"hello world 1");
    assert_ok_eq!(client2.recv().await, b"hello world 2");
}

#[tokio::test]
async fn server_compat() {
    subscribe_tracing!();

    let server = Server::default();

    let addr = server.listen("tcp://127.0.0.1:0").await.unwrap();
    let address: libzmq::TcpAddr = match addr {
        rmq::Endpoint::Tcp(addr) => addr.to_string().try_into().unwrap(),
        _ => panic!("unexpected proto"),
    };
    let endpoint: libzmq::addr::Endpoint = address.into();

    let client1 = libzmq::ClientBuilder::new()
        .connect(&endpoint)
        .build()
        .unwrap();

    let client2 = libzmq::ClientBuilder::new()
        .connect(&endpoint)
        .build()
        .unwrap();

    thread::spawn(move || {
        assert_ok!(client1.send("hello 1"));
        assert_ok!(client2.send("hello 2"));
        assert_eq!(client1.recv_msg().unwrap().as_bytes(), b"hello world 1");
        assert_eq!(client2.recv_msg().unwrap().as_bytes(), b"hello world 2");
    });

    tokio::time::delay_for(std::time::Duration::from_millis(1)).await;

    let msg1 = server.recv().await.unwrap();
    assert_eq!(msg1.as_bytes(), b"hello 1");

    let msg2 = server.recv().await.unwrap();
    assert_eq!(msg2.as_bytes(), b"hello 2");

    server
        .route(&b"hello world 1"[..], msg1.route)
        .await
        .unwrap();
    server
        .route(&b"hello world 2"[..], msg2.route)
        .await
        .unwrap();
}
