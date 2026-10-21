#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_count_on_sender_constructor() {
        let sender = Sender::<i32>::new(16);
        assert_eq!(sender.receiver_count(), 0);

        let rx_1 = sender.subscribe();
        assert_eq!(sender.receiver_count(), 1);

        let rx_2 = rx_1.resubscribe();
        assert_eq!(sender.receiver_count(), 2);

        let rx_3 = sender.subscribe();
        assert_eq!(sender.receiver_count(), 3);

        drop(rx_3);
        drop(rx_1);
        assert_eq!(sender.receiver_count(), 1);

        drop(rx_2);
        assert_eq!(sender.receiver_count(), 0);
    }


    #[test]
    fn receiver_count_on_channel_constructor() {
        let (sender, rx) = channel::<i32>(16);
        assert_eq!(sender.receiver_count(), 1);

        let _rx_2 = rx.resubscribe();
        assert_eq!(sender.receiver_count(), 2);
    }
}
