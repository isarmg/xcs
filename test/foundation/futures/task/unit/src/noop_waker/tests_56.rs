#[cfg(test)]
mod tests {
    #[test]
    #[cfg(feature = "std")]
    fn issue_2091_cross_thread_segfault() {
        let waker = std::thread::spawn(super::noop_waker_ref).join().unwrap();
        waker.wake_by_ref();
    }
}
