#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future,
        task::{Context, Poll},
    };

    struct Svc;
    impl Service<()> for Svc {
        type Response = ();
        type Error = ();
        type Future = future::Ready<Result<(), ()>>;

        fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), ()>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, (): ()) -> Self::Future {
            future::ready(Ok(()))
        }
    }

    #[test]
    fn default() {
        let mut svc = PendingRequests::new(Svc, CompleteOnResponse);
        assert_eq!(svc.load(), Count(0));

        let rsp0 = svc.call(());
        assert_eq!(svc.load(), Count(1));

        let rsp1 = svc.call(());
        assert_eq!(svc.load(), Count(2));

        let () = tokio_test::block_on(rsp0).unwrap();
        assert_eq!(svc.load(), Count(1));

        let () = tokio_test::block_on(rsp1).unwrap();
        assert_eq!(svc.load(), Count(0));
    }

    #[test]
    fn with_completion() {
        #[derive(Clone)]
        struct IntoHandle;
        impl TrackCompletion<Handle, ()> for IntoHandle {
            type Output = Handle;
            fn track_completion(&self, i: Handle, (): ()) -> Handle {
                i
            }
        }

        let mut svc = PendingRequests::new(Svc, IntoHandle);
        assert_eq!(svc.load(), Count(0));

        let rsp = svc.call(());
        assert_eq!(svc.load(), Count(1));
        let i0 = tokio_test::block_on(rsp).unwrap();
        assert_eq!(svc.load(), Count(1));

        let rsp = svc.call(());
        assert_eq!(svc.load(), Count(2));
        let i1 = tokio_test::block_on(rsp).unwrap();
        assert_eq!(svc.load(), Count(2));

        drop(i1);
        assert_eq!(svc.load(), Count(1));

        drop(i0);
        assert_eq!(svc.load(), Count(0));
    }
}
