#[cfg(test)]
mod tests {
	use std::ops::Deref;
	use std::rc::Rc;
	use std::sync::mpsc::channel;
	use std::sync::Arc;
	use std::thread;

	use super::SendWrapper;

	#[test]
	fn test_deref() {
		let (sender, receiver) = channel();
		let w = SendWrapper::new(Rc::new(42));
		{
			let _x = w.deref();
		}
		let t = thread::spawn(move || {
			// move SendWrapper back to main thread, so it can be dropped from there
			sender.send(w).unwrap();
		});
		let w2 = receiver.recv().unwrap();
		{
			let _x = w2.deref();
		}
		assert!(t.join().is_ok());
	}

	#[test]
	fn test_deref_panic() {
		let w = SendWrapper::new(Rc::new(42));
		let t = thread::spawn(move || {
			let _x = w.deref();
		});
		let join_result = t.join();
		assert!(join_result.is_err());
	}

	#[test]
	fn test_drop_panic() {
		let w = SendWrapper::new(Rc::new(42));
		let t = thread::spawn(move || {
			let _x = w;
		});
		let join_result = t.join();
		assert!(join_result.is_err());
	}

	#[test]
	fn test_valid() {
		let w = SendWrapper::new(Rc::new(42));
		assert!(w.valid());
		thread::spawn(move || {
			assert!(!w.valid());
		});
	}

	#[test]
	fn test_take() {
		let w = SendWrapper::new(Rc::new(42));
		let inner: Rc<usize> = w.take();
		assert_eq!(42, *inner);
	}

	#[test]
	fn test_take_panic() {
		let w = SendWrapper::new(Rc::new(42));
		let t = thread::spawn(move || {
			let _ = w.take();
		});
		assert!(t.join().is_err());
	}

	#[test]
	fn test_sync() {
		// Arc<T> can only be sent to another thread if T Sync
		let arc = Arc::new(SendWrapper::new(42));
		thread::spawn(move || {
			let _ = arc;
		});
	}

	#[test]
	fn test_debug() {
		let w = SendWrapper::new(Rc::new(42));
		let info = format!("{:?}", w);
		assert!(info.contains("SendWrapper {"));
		assert!(info.contains("data: 42,"));
		assert!(info.contains("thread_id: ThreadId("));
	}

	#[test]
	fn test_debug_panic() {
		let w = SendWrapper::new(Rc::new(42));
		let t = thread::spawn(move || {
			let _ = format!("{:?}", w);
		});
		assert!(t.join().is_err());
	}

	#[test]
	fn test_clone() {
		let w1 = SendWrapper::new(Rc::new(42));
		let w2 = w1.clone();
		assert_eq!(format!("{:?}", w1), format!("{:?}", w2));
	}

	#[test]
	fn test_clone_panic() {
		let w = SendWrapper::new(Rc::new(42));
		let t = thread::spawn(move || {
			let _ = w.clone();
		});
		assert!(t.join().is_err());
	}
}
