use rmq::{Client, Server};

mod test;
use claim::*;

#[tokio::test]
async fn client_server_routing() {
    subscribe_tracing!();

    for transport in test::transports() {
        let addr = test::endpoint(transport);

        let c1 = Client::default();
        assert_ok!(c1.connect(&addr).await);

        let c2 = Client::default();
        assert_ok!(c2.connect(&addr).await);

        let s = Server::default();
        assert_ok!(s.listen(&addr).await);

        assert_ok!(c1.send("hello 1").await);
        let msg1 = s.recv().await;
        assert_ok_eq!(&msg1, b"hello 1");
        let id1 = msg1.unwrap().route;

        assert_ok!(c2.send("hello 2").await);
        let msg2 = s.recv().await;
        assert_ok_eq!(&msg2, b"hello 2");
        let id2 = msg2.unwrap().route;

        assert_ok!(s.route("hello 1", id1).await);
        assert_ok!(s.route("hello 2", id2).await);

        let msg1 = c1.recv().await.unwrap();
        assert_eq!(msg1, b"hello 1");

        let msg2 = c2.recv().await.unwrap();
        assert_eq!(msg2, b"hello 2");

        assert_ne!(msg1.route, msg2.route);
    }
}
