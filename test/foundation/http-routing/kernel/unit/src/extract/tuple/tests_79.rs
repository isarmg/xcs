#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http::Method;

    use crate::extract::{FromRequest, FromRequestParts};

    fn assert_from_request<M, T>()
    where
        T: FromRequest<(), M>,
    {
    }

    fn assert_from_request_parts<T: FromRequestParts<()>>() {}

    #[test]
    fn unit() {
        assert_from_request_parts::<()>();
        assert_from_request::<_, ()>();
    }

    #[test]
    fn tuple_of_one() {
        assert_from_request_parts::<(Method,)>();
        assert_from_request::<_, (Method,)>();
        assert_from_request::<_, (Bytes,)>();
    }

    #[test]
    fn tuple_of_two() {
        assert_from_request_parts::<((), ())>();
        assert_from_request::<_, ((), ())>();
        assert_from_request::<_, (Method, Bytes)>();
    }

    #[test]
    fn nested_tuple() {
        assert_from_request_parts::<(((Method,),),)>();
        assert_from_request::<_, ((((Bytes,),),),)>();
    }
}
