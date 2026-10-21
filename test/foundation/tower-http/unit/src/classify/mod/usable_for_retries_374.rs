#[cfg(test)]
mod usable_for_retries {
    #![allow(dead_code)]

    use std::fmt;

    use http::{Request, Response};
    use tower::retry::Policy;

    use super::{ClassifiedResponse, ClassifyResponse};

    trait IsRetryable {
        fn is_retryable(&self) -> bool;
    }

    #[derive(Clone)]
    struct RetryBasedOnClassification<C> {
        classifier: C,
        // ...
    }

    impl<ReqB, ResB, E, C> Policy<Request<ReqB>, Response<ResB>, E> for RetryBasedOnClassification<C>
    where
        C: ClassifyResponse + Clone,
        E: fmt::Display + 'static,
        C::FailureClass: IsRetryable,
        ResB: http_body::Body,
        Request<ReqB>: Clone,
        E: std::error::Error + 'static,
    {
        type Future = std::future::Ready<()>;

        fn retry(
            &mut self,
            _req: &mut Request<ReqB>,
            res: &mut Result<Response<ResB>, E>,
        ) -> Option<Self::Future> {
            match res {
                Ok(res) => {
                    if let ClassifiedResponse::Ready(class) =
                        self.classifier.clone().classify_response(res)
                    {
                        if class.err()?.is_retryable() {
                            return Some(std::future::ready(()));
                        }
                    }

                    None
                }
                Err(err) => self
                    .classifier
                    .clone()
                    .classify_error(err)
                    .is_retryable()
                    .then(|| std::future::ready(())),
            }
        }

        fn clone_request(&mut self, req: &Request<ReqB>) -> Option<Request<ReqB>> {
            Some(req.clone())
        }
    }
}
