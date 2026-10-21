#[cfg(test)]
mod tests {
	use std::thread;

	use futures::executor;
	use futures::{future, stream, StreamExt};

	use crate::SendWrapper;

	#[test]
	fn test_future() {
		let w1 = SendWrapper::new(future::ready(42));
		let w2 = w1.clone();
		assert_eq!(
			format!("{:?}", executor::block_on(w1)),
			format!("{:?}", executor::block_on(w2)),
		);
	}

	#[test]
	fn test_future_panic() {
		let w = SendWrapper::new(future::ready(42));
		let t = thread::spawn(move || executor::block_on(w));
		assert!(t.join().is_err());
	}

	#[test]
	fn test_stream() {
		let mut w1 = SendWrapper::new(stream::once(future::ready(42)));
		let mut w2 = SendWrapper::new(stream::once(future::ready(42)));
		assert_eq!(
			format!("{:?}", executor::block_on(w1.next())),
			format!("{:?}", executor::block_on(w2.next())),
		);
	}

	#[test]
	fn test_stream_panic() {
		let mut w = SendWrapper::new(stream::once(future::ready(42)));
		let t = thread::spawn(move || executor::block_on(w.next()));
		assert!(t.join().is_err());
	}
}
