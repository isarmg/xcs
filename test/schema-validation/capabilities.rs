use json_codec::{json, Value};
use schema_validation::{AsyncRetrieve, Uri};
struct Memory;
#[async_trait::async_trait]
impl AsyncRetrieve for Memory {
    async fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut yielded = false;
        std::future::poll_fn(|context| {
            if yielded {
                std::task::Poll::Ready(())
            } else {
                yielded = true;
                context.waker().wake_by_ref();
                std::task::Poll::Pending
            }
        })
        .await;
        assert_eq!(uri.as_str(), "https://example.test/schema");
        Ok(json!({"type":"integer", "minimum":10}))
    }
}
#[test]
fn actual_async_retrieval_and_resolution() {
    block_on(async {
        let schema = json!({"$ref":"https://example.test/schema"});
        assert!(schema_validation::validator_for(&schema).is_err());
        let v = schema_validation::async_options()
            .with_retriever(Memory)
            .build(&schema)
            .await
            .unwrap();
        assert!(v.is_valid(&json!(12)));
        assert!(!v.is_valid(&json!(3)));
        assert!(!v.is_valid(&json!("12")));
    });
}
#[test]
fn typed_jsonb_validation() {
    use schema_validation::values::{jsonb::encode::encode, Jsonb};
    let v = schema_validation::options_for::<Jsonb>()
        .build(&json!({"type":"integer","minimum":10}))
        .unwrap();
    let good = encode(&json!(12));
    let bad = encode(&json!(3));
    assert!(v.is_valid(Jsonb::root(&good)));
    assert!(!v.is_valid(Jsonb::root(&bad)));
}
#[test]
fn default_rejects_file_and_http_retrieval() {
    for uri in [
        "file:///nonexistent/schema.json",
        "https://example.test/schema",
    ] {
        assert!(schema_validation::validator_for(&json!({"$ref": uri})).is_err());
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    use std::{
        sync::Arc,
        task::{Context, Poll, Wake, Waker},
    };
    struct ThreadWake(std::thread::Thread);
    impl Wake for ThreadWake {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(v) => return v,
            Poll::Pending => std::thread::park(),
        }
    }
}
