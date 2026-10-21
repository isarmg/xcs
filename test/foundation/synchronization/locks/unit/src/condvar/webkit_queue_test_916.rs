#[cfg(test)]
mod webkit_queue_test {
    use crate::locks::{Condvar, Mutex, MutexGuard};
    use std::{collections::VecDeque, sync::Arc, thread, time::Duration};

    #[derive(Clone, Copy)]
    enum Timeout {
        Bounded(Duration),
        Forever,
    }

    #[derive(Clone, Copy)]
    enum NotifyStyle {
        One,
        All,
    }

    struct Queue {
        items: VecDeque<usize>,
        should_continue: bool,
    }

    impl Queue {
        fn new() -> Self {
            Self {
                items: VecDeque::new(),
                should_continue: true,
            }
        }
    }

    fn wait<T: ?Sized>(
        condition: &Condvar,
        lock: &mut MutexGuard<'_, T>,
        predicate: impl Fn(&mut MutexGuard<'_, T>) -> bool,
        timeout: &Timeout,
    ) {
        while !predicate(lock) {
            match timeout {
                Timeout::Forever => condition.wait(lock),
                Timeout::Bounded(bound) => {
                    condition.wait_for(lock, *bound);
                }
            }
        }
    }

    fn notify(style: NotifyStyle, condition: &Condvar, should_notify: bool) {
        match style {
            NotifyStyle::One => {
                condition.notify_one();
            }
            NotifyStyle::All => {
                if should_notify {
                    condition.notify_all();
                }
            }
        }
    }

    fn run_queue_test(
        num_producers: usize,
        num_consumers: usize,
        max_queue_size: usize,
        messages_per_producer: usize,
        notify_style: NotifyStyle,
        timeout: Timeout,
        delay: Duration,
    ) {
        let input_queue = Arc::new(Mutex::new(Queue::new()));
        let empty_condition = Arc::new(Condvar::new());
        let full_condition = Arc::new(Condvar::new());

        let output_vec = Arc::new(Mutex::new(vec![]));

        let consumers = (0..num_consumers)
            .map(|_| {
                consumer_thread(
                    input_queue.clone(),
                    empty_condition.clone(),
                    full_condition.clone(),
                    timeout,
                    notify_style,
                    output_vec.clone(),
                    max_queue_size,
                )
            })
            .collect::<Vec<_>>();
        let producers = (0..num_producers)
            .map(|_| {
                producer_thread(
                    messages_per_producer,
                    input_queue.clone(),
                    empty_condition.clone(),
                    full_condition.clone(),
                    timeout,
                    notify_style,
                    max_queue_size,
                )
            })
            .collect::<Vec<_>>();

        thread::sleep(delay);

        for producer in producers.into_iter() {
            producer.join().expect("Producer thread panicked");
        }

        {
            let mut input_queue = input_queue.lock();
            input_queue.should_continue = false;
        }
        empty_condition.notify_all();

        for consumer in consumers.into_iter() {
            consumer.join().expect("Consumer thread panicked");
        }

        let mut output_vec = output_vec.lock();
        assert_eq!(output_vec.len(), num_producers * messages_per_producer);
        output_vec.sort();
        for msg_idx in 0..messages_per_producer {
            for producer_idx in 0..num_producers {
                assert_eq!(msg_idx, output_vec[msg_idx * num_producers + producer_idx]);
            }
        }
    }

    fn consumer_thread(
        input_queue: Arc<Mutex<Queue>>,
        empty_condition: Arc<Condvar>,
        full_condition: Arc<Condvar>,
        timeout: Timeout,
        notify_style: NotifyStyle,
        output_queue: Arc<Mutex<Vec<usize>>>,
        max_queue_size: usize,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || loop {
            let (should_notify, result) = {
                let mut queue = input_queue.lock();
                wait(
                    &empty_condition,
                    &mut queue,
                    |state| -> bool { !state.items.is_empty() || !state.should_continue },
                    &timeout,
                );
                if queue.items.is_empty() && !queue.should_continue {
                    return;
                }
                let should_notify = queue.items.len() == max_queue_size;
                let result = queue.items.pop_front();
                std::mem::drop(queue);
                (should_notify, result)
            };
            notify(notify_style, &full_condition, should_notify);

            if let Some(result) = result {
                output_queue.lock().push(result);
            }
        })
    }

    fn producer_thread(
        num_messages: usize,
        queue: Arc<Mutex<Queue>>,
        empty_condition: Arc<Condvar>,
        full_condition: Arc<Condvar>,
        timeout: Timeout,
        notify_style: NotifyStyle,
        max_queue_size: usize,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            for message in 0..num_messages {
                let should_notify = {
                    let mut queue = queue.lock();
                    wait(
                        &full_condition,
                        &mut queue,
                        |state| state.items.len() < max_queue_size,
                        &timeout,
                    );
                    let should_notify = queue.items.is_empty();
                    queue.items.push_back(message);
                    std::mem::drop(queue);
                    should_notify
                };
                notify(notify_style, &empty_condition, should_notify);
            }
        })
    }

    macro_rules! run_queue_tests {
        ( $( $name:ident(
            num_producers: $num_producers:expr,
            num_consumers: $num_consumers:expr,
            max_queue_size: $max_queue_size:expr,
            messages_per_producer: $messages_per_producer:expr,
            notification_style: $notification_style:expr,
            timeout: $timeout:expr,
            delay_seconds: $delay_seconds:expr);
        )* ) => {
            $(#[test]
            fn $name() {
                let delay = Duration::from_secs($delay_seconds);
                run_queue_test(
                    $num_producers,
                    $num_consumers,
                    $max_queue_size,
                    $messages_per_producer,
                    $notification_style,
                    $timeout,
                    delay,
                    );
            })*
        };
    }

    run_queue_tests! {
        sanity_check_queue(
            num_producers: 1,
            num_consumers: 1,
            max_queue_size: 1,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Bounded(Duration::from_secs(1)),
            delay_seconds: 0
        );
        sanity_check_queue_timeout(
            num_producers: 1,
            num_consumers: 1,
            max_queue_size: 1,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        new_test_without_timeout_5(
            num_producers: 1,
            num_consumers: 5,
            max_queue_size: 1,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        one_producer_one_consumer_one_slot(
            num_producers: 1,
            num_consumers: 1,
            max_queue_size: 1,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        one_producer_one_consumer_one_slot_timeout(
            num_producers: 1,
            num_consumers: 1,
            max_queue_size: 1,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 1
        );
        one_producer_one_consumer_hundred_slots(
            num_producers: 1,
            num_consumers: 1,
            max_queue_size: 100,
            messages_per_producer: 1_000_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        ten_producers_one_consumer_one_slot(
            num_producers: 10,
            num_consumers: 1,
            max_queue_size: 1,
            messages_per_producer: 10000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        ten_producers_one_consumer_hundred_slots_notify_all(
            num_producers: 10,
            num_consumers: 1,
            max_queue_size: 100,
            messages_per_producer: 10000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        ten_producers_one_consumer_hundred_slots_notify_one(
            num_producers: 10,
            num_consumers: 1,
            max_queue_size: 100,
            messages_per_producer: 10000,
            notification_style: NotifyStyle::One,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        one_producer_ten_consumers_one_slot(
            num_producers: 1,
            num_consumers: 10,
            max_queue_size: 1,
            messages_per_producer: 10000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        one_producer_ten_consumers_hundred_slots_notify_all(
            num_producers: 1,
            num_consumers: 10,
            max_queue_size: 100,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        one_producer_ten_consumers_hundred_slots_notify_one(
            num_producers: 1,
            num_consumers: 10,
            max_queue_size: 100,
            messages_per_producer: 100_000,
            notification_style: NotifyStyle::One,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        ten_producers_ten_consumers_one_slot(
            num_producers: 10,
            num_consumers: 10,
            max_queue_size: 1,
            messages_per_producer: 50000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        ten_producers_ten_consumers_hundred_slots_notify_all(
            num_producers: 10,
            num_consumers: 10,
            max_queue_size: 100,
            messages_per_producer: 50000,
            notification_style: NotifyStyle::All,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
        ten_producers_ten_consumers_hundred_slots_notify_one(
            num_producers: 10,
            num_consumers: 10,
            max_queue_size: 100,
            messages_per_producer: 50000,
            notification_style: NotifyStyle::One,
            timeout: Timeout::Forever,
            delay_seconds: 0
        );
    }
}
