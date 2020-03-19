// use rmq::{Outgoing, Incoming, Server, Client};

// #[tokio::main]
// async fn main() {
//     let mut sock = Client::default();
//     sock.listen("tcp://localhost:0").await;

//     while let Ok(msg) = sock.recv().await {
//         let id = msg.routing_id();
//         msg.peer_address();

//         sock.broadcast("foobar", b"1231213");

//         msg.group = b"asfasfs";

//         sock.send(msg).await;
//         sock.send("foo", msg).await;
//         sock.send(233412, msg).await;
//     }
// }
fn main() {}
