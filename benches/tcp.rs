use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use rmq::{Client, Server};

async fn rmq_server_client_ipv4<'a>(msg: &'static [u8], a: &'a Client, b: &'a Server) {
    for _ in 0..1000 {
        a.send(msg).await.unwrap();
    }

    for _ in 0..1000 {
        b.recv().await.unwrap();
    }
}

fn zmq_server_client_ipv4<'a>(
    msg: &'static [u8],
    a: &'a mut libzmq::Client,
    b: &'a mut libzmq::Server,
) {
    use libzmq::prelude::{RecvMsg, SendMsg};

    for _ in 0..1000 {
        a.send(msg).unwrap();
    }

    for _ in 0..1000 {
        b.recv_msg().unwrap();
    }
}

fn cfg_bench(c: &mut Criterion) {
    const MSG: &[u8] = b"Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world!";

    let mut rt = tokio::runtime::Runtime::new().expect("runtime");
    let (server, client) = rt.block_on(async {
        let (server, client): (rmq::Server, rmq::Client) = Default::default();
        let addr = server.listen("tcp://127.0.0.1:0").await.unwrap();
        client.connect(addr).await.unwrap();
        (server, client)
    });

    c.bench_with_input(
        BenchmarkId::new("rmq server/client", "ipv4"),
        &MSG,
        |b, s| {
            b.iter(|| {
                rt.block_on(rmq_server_client_ipv4(s, &client, &server));
            })
        },
    );

    use libzmq::prelude::{BuildSocket, Socket, TryInto};
    let address: libzmq::TcpAddr = "127.0.0.1:*".try_into().expect("addr");
    let endpoint: libzmq::addr::Endpoint = address.into();
    let mut server = libzmq::ServerBuilder::new().bind(endpoint).build().unwrap();
    let endpoint = server.last_endpoint();
    let mut client = libzmq::ClientBuilder::new()
        .connect(endpoint)
        .build()
        .unwrap();

    c.bench_with_input(
        BenchmarkId::new("zmq server/client", "ipv4"),
        &MSG,
        |b, s| {
            b.iter(|| {
                zmq_server_client_ipv4(s, &mut client, &mut server);
            })
        },
    );
}

criterion_group!(benches, cfg_bench);
criterion_main!(benches);
