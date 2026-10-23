#![feature(test)]

extern crate test;
#[path = "../../../src/common/try_lock.rs"]
mod try_lock;
#[path = "../../../src/common/demand.rs"]
mod demand;

#[bench]
fn throughput(b: &mut test::Bencher) {
    let (mut gv, mut tk) = demand::new();

    b.iter(move || {
        tk.want();
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(matches!(gv.poll_want(&mut cx), std::task::Poll::Ready(Ok(()))));
    });
}
