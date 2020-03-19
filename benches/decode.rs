use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

// use futures::Stream;

// mod private {
//     #![allow(warnings)]

//     macro_rules! trace {
//         ($($arg:tt)+) => {}
//     }

//     include!("../src/zmtp/mod.rs");
// }

// use private::*;

// fn decode(buf: &[u8]) {
//     tokio::runtime::Runtime::new()
//         .expect("runtime")
//         .block_on(async {
//             futures::future::poll_fn(move |cx| {
//                 tokio::pin! {
//                     let reader = Reader::new(buf, Zmtp::default());
//                 };

//                 reader.poll_next(cx)
//             })
//             .await;
//         });
// }

fn cfg_bench(c: &mut Criterion) {
    //     const MESSAGE: &[u8] = b"\x01\x0cHello world!\xff";
    //     const LARGE_MESSAGE: &[u8] = b"\x01\0\0\0\0\0\0\x01\x38Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! Hello world! \xff";
    //     const GREETING: &[u8] = b"\xff\0\0\0\0\0\0\0\0\x7f\x03\x01NULL\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
    //     const READY: &[u8] = b"\x04\x1c\x05READY\x0bSocket-Type\0\0\0\x06ROUTER\xff";
    //     const PING: &[u8] = b"\x04\x08\x04PING\x01\x7fa\xff";

    //     c.bench_with_input(BenchmarkId::new("decode", "message"), &MESSAGE, |b, s| {
    //         b.iter(|| decode(s))
    //     });

    //     c.bench_with_input(
    //         BenchmarkId::new("decode", "large message"),
    //         &LARGE_MESSAGE,
    //         |b, s| b.iter(|| decode(s)),
    //     );

    //     c.bench_with_input(BenchmarkId::new("decode", "greeting"), &GREETING, |b, s| {
    //         b.iter(|| decode(s))
    //     });

    //     c.bench_with_input(BenchmarkId::new("decode", "ready"), &READY, |b, s| {
    //         b.iter(|| decode(s))
    //     });

    //     c.bench_with_input(BenchmarkId::new("decode", "ping"), &PING, |b, s| {
    //         b.iter(|| decode(s))
    //     });
}

criterion_group!(benches, cfg_bench);
criterion_main!(benches);
