#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::{future_service, ServiceExt};
    use crate::Service;
    use std::{
        convert::Infallible,
        future::{ready, Ready},
    };

    #[tokio::test]
    async fn pending_service_debug_impl() {
        let mut pending_svc = future_service(ready(Ok(DebugService)));

        assert_eq!(
            format!("{pending_svc:?}"),
            "FutureService { state: State::Future(<core::future::ready::Ready<core::result::Result<tower::util::future_service::tests::DebugService, core::convert::Infallible>>>) }"
        );

        pending_svc.ready().await.unwrap();

        assert_eq!(
            format!("{pending_svc:?}"),
            "FutureService { state: State::Service(DebugService) }"
        );
    }

    #[derive(Debug)]
    struct DebugService;

    impl Service<()> for DebugService {
        type Response = ();
        type Error = Infallible;
        type Future = Ready<Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Ok(()).into()
        }

        fn call(&mut self, _req: ()) -> Self::Future {
            ready(Ok(()))
        }
    }
}
