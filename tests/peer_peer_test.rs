use rmq::Peer;

mod test;
use claim::*;

#[tokio::test]
async fn peer_peer_routing() {
    subscribe_tracing!();

    for transport in test::transports() {
        let p1 = Peer::default();
        let addr1 = p1.listen(test::endpoint(transport)).await.unwrap();

        let p2 = Peer::default();
        let addr2 = p2.listen(test::endpoint(transport)).await.unwrap();

        let p3 = Peer::default();
        let addr3 = p3.listen(test::endpoint(transport)).await.unwrap();

        let id1_2 = p1.connect(&addr2).await.unwrap();
        let id1_3 = p1.connect(&addr3).await.unwrap();

        let id2_1 = p2.connect(&addr1).await.unwrap();
        let id2_3 = p2.connect(&addr3).await.unwrap();

        let id3_1 = p3.connect(&addr1).await.unwrap();
        let id3_2 = p3.connect(&addr2).await.unwrap();

        assert_ok!(p1.route("hello from 1", id1_2).await);
        assert_ok!(p1.route("hello from 1", id1_3).await);

        assert_ok!(p2.route("hello from 2", id2_1).await);
        assert_ok!(p2.route("hello from 2", id2_3).await);

        assert_ok!(p3.route("hello from 3", id3_1).await);
        assert_ok!(p3.route("hello from 3", id3_2).await);

        let mut recv1 = vec![
            p1.recv().await.unwrap().into_bytes(),
            p1.recv().await.unwrap().into_bytes(),
        ];
        recv1.sort();

        let mut recv2 = vec![
            p2.recv().await.unwrap().into_bytes(),
            p2.recv().await.unwrap().into_bytes(),
        ];
        recv2.sort();

        let mut recv3 = vec![
            p3.recv().await.unwrap().into_bytes(),
            p3.recv().await.unwrap().into_bytes(),
        ];
        recv3.sort();

        assert_eq!(
            vec![b"hello from 2", b"hello from 3"],
            recv1.iter().map(bytes::Bytes::as_ref).collect::<Vec<_>>(),
        );

        assert_eq!(
            vec![b"hello from 1", b"hello from 3"],
            recv2.iter().map(bytes::Bytes::as_ref).collect::<Vec<_>>(),
        );

        assert_eq!(
            vec![b"hello from 1", b"hello from 2"],
            recv3.iter().map(bytes::Bytes::as_ref).collect::<Vec<_>>(),
        );
    }
}
