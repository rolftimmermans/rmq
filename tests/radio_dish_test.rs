use rmq::{Dish, Radio};

mod test;
use claim::*;

#[tokio::test]
async fn radio_dish_broadcasting() {
    subscribe_tracing!();

    for transport in test::transports() {
        let radio = Radio::default();
        let addr = radio.listen(test::endpoint(transport)).await.unwrap();

        let blank_group = "".parse().unwrap();
        let foo_group = "foo".parse().unwrap();
        let bar_group = "bar".parse().unwrap();

        let dish1 = Dish::default();
        assert_ok!(dish1.connect(&addr).await);
        dish1.join(foo_group);

        let dish2 = Dish::default();
        assert_ok!(dish2.connect(&addr).await);
        dish2.join(blank_group);
        dish2.join(bar_group);

        tokio::time::delay_for(std::time::Duration::from_millis(50)).await;

        radio.broadcast("hello foo", foo_group).unwrap();
        radio.broadcast("hello bar", bar_group).unwrap();
        radio.broadcast("hello", blank_group).unwrap();

        assert_ok_eq!(dish1.recv().await, b"hello foo");
        assert_ok_eq!(dish1.recv().await, b"hello bar"); // Process subscriptions
        assert_ok_eq!(dish1.recv().await, b"hello");
        assert_ok_eq!(dish2.recv().await, b"hello foo");
        assert_ok_eq!(dish2.recv().await, b"hello bar");
        assert_ok_eq!(dish2.recv().await, b"hello");
    }
}
