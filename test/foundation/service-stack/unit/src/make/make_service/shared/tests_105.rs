#[cfg(test)]
mod tests {
    use super::*;
    use crate::make::MakeService;
    use crate::service_fn;
    use std::future::poll_fn;

    async fn echo<R>(req: R) -> Result<R, Infallible> {
        Ok(req)
    }

    #[tokio::test]
    async fn as_make_service() {
        let mut shared = Shared::new(service_fn(echo::<&'static str>));

        poll_fn(|cx| MakeService::<(), _>::poll_ready(&mut shared, cx))
            .await
            .unwrap();
        let mut svc = shared.make_service(()).await.unwrap();

        poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
        let res = svc.call("foo").await.unwrap();

        assert_eq!(res, "foo");
    }

    #[tokio::test]
    async fn as_make_service_into_service() {
        let shared = Shared::new(service_fn(echo::<&'static str>));
        let mut shared = MakeService::<(), _>::into_service(shared);

        poll_fn(|cx| Service::<()>::poll_ready(&mut shared, cx))
            .await
            .unwrap();
        let mut svc = shared.call(()).await.unwrap();

        poll_fn(|cx| svc.poll_ready(cx)).await.unwrap();
        let res = svc.call("foo").await.unwrap();

        assert_eq!(res, "foo");
    }
}
