#[cfg(test)]
mod tests {
    use super::*;
    use crate::{routing::get, test_helpers::*, Router};
    use data_serialization::Deserialize;
    use std::collections::HashMap;

    #[crate::test]
    async fn extracting_url_params() {
        let app = Router::new().route(
            "/users/{id}",
            get(|Path(id): Path<i32>| async move {
                assert_eq!(id, 42);
            })
            .post(|Path(params_map): Path<HashMap<String, i32>>| async move {
                assert_eq!(params_map.get("id").unwrap(), &1337);
            }),
        );

        let client = TestClient::new(app);

        let res = client.get("/users/42").await;
        assert_eq!(res.status(), StatusCode::OK);

        let res = client.post("/users/1337").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn extracting_url_params_multiple_times() {
        let app = Router::new().route("/users/{id}", get(|_: Path<i32>, _: Path<String>| async {}));

        let client = TestClient::new(app);

        let res = client.get("/users/42").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn percent_decoding() {
        let app = Router::new().route(
            "/{key}",
            get(|Path(param): Path<String>| async move { param }),
        );

        let client = TestClient::new(app);

        let res = client.get("/one%20two").await;

        assert_eq!(res.text().await, "one two");
    }

    #[crate::test]
    async fn supports_128_bit_numbers() {
        let app = Router::new()
            .route(
                "/i/{key}",
                get(|Path(param): Path<i128>| async move { param.to_string() }),
            )
            .route(
                "/u/{key}",
                get(|Path(param): Path<u128>| async move { param.to_string() }),
            );

        let client = TestClient::new(app);

        let res = client.get("/i/123").await;
        assert_eq!(res.text().await, "123");

        let res = client.get("/u/123").await;
        assert_eq!(res.text().await, "123");
    }

    #[crate::test]
    async fn wildcard() {
        let app = Router::new()
            .route(
                "/foo/{*rest}",
                get(|Path(param): Path<String>| async move { param }),
            )
            .route(
                "/bar/{*rest}",
                get(|Path(params): Path<HashMap<String, String>>| async move {
                    params.get("rest").unwrap().clone()
                }),
            );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar/baz").await;
        assert_eq!(res.text().await, "bar/baz");

        let res = client.get("/bar/baz/qux").await;
        assert_eq!(res.text().await, "baz/qux");
    }

    #[crate::test]
    async fn captures_dont_match_empty_path() {
        let app = Router::new().route("/{key}", get(|| async {}));

        let client = TestClient::new(app);

        let res = client.get("/").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let res = client.get("/foo").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn captures_match_empty_inner_segments() {
        let app = Router::new().route(
            "/{key}/method",
            get(|Path(param): Path<String>| async move { param.clone() }),
        );

        let client = TestClient::new(app);

        let res = client.get("/abc/method").await;
        assert_eq!(res.text().await, "abc");

        let res = client.get("//method").await;
        assert_eq!(res.text().await, "");
    }

    #[crate::test]
    async fn captures_match_empty_inner_segments_near_end() {
        let app = Router::new().route(
            "/method/{key}/",
            get(|Path(param): Path<String>| async move { param.clone() }),
        );

        let client = TestClient::new(app);

        let res = client.get("/method/abc").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let res = client.get("/method/abc/").await;
        assert_eq!(res.text().await, "abc");

        let res = client.get("/method//").await;
        assert_eq!(res.text().await, "");
    }

    #[crate::test]
    async fn captures_match_empty_trailing_segment() {
        let app = Router::new().route(
            "/method/{key}",
            get(|Path(param): Path<String>| async move { param.clone() }),
        );

        let client = TestClient::new(app);

        let res = client.get("/method/abc/").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);

        let res = client.get("/method/abc").await;
        assert_eq!(res.text().await, "abc");

        let res = client.get("/method/").await;
        assert_eq!(res.text().await, "");

        let res = client.get("/method").await;
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
    }

    #[crate::test]
    async fn str_reference_deserialize() {
        struct Param(String);
        impl<'de> data_serialization::Deserialize<'de> for Param {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: data_serialization::Deserializer<'de>,
            {
                let s = <&str as data_serialization::Deserialize>::deserialize(deserializer)?;
                Ok(Param(s.to_owned()))
            }
        }

        let app = Router::new().route(
            "/{key}",
            get(|param: Path<Param>| async move { param.0 .0 }),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo").await;
        assert_eq!(res.text().await, "foo");

        // percent decoding should also work
        let res = client.get("/foo%20bar").await;
        assert_eq!(res.text().await, "foo bar");
    }

    #[crate::test]
    async fn two_path_extractors() {
        let app = Router::new().route("/{a}/{b}", get(|_: Path<String>, _: Path<String>| async {}));

        let client = TestClient::new(app);

        let res = client.get("/a/b").await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            res.text().await,
            "Wrong number of path arguments for `Path`. Expected 1 but got 2. \
            Note that multiple parameters must be extracted with a tuple `Path<(_, _)>` or a struct `Path<YourParams>`",
        );
    }

    #[crate::test]
    async fn tuple_param_matches_exactly() {
        #[allow(dead_code)]
        #[derive(Deserialize)]
        struct Tuple(String, String);

        let app = Router::new()
            .route(
                "/foo/{a}/{b}/{c}",
                get(|_: Path<(String, String)>| async {}),
            )
            .route("/bar/{a}/{b}/{c}", get(|_: Path<Tuple>| async {}));

        let client = TestClient::new(app);

        let res = client.get("/foo/a/b/c").await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            res.text().await,
            "Wrong number of path arguments for `Path`. Expected 2 but got 3",
        );

        let res = client.get("/bar/a/b/c").await;
        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            res.text().await,
            "Wrong number of path arguments for `Path`. Expected 2 but got 3",
        );
    }

    #[crate::test]
    async fn deserialize_into_vec_of_tuples() {
        let app = Router::new().route(
            "/{a}/{b}",
            get(|Path(params): Path<Vec<(String, String)>>| async move {
                assert_eq!(
                    params,
                    vec![
                        ("a".to_owned(), "foo".to_owned()),
                        ("b".to_owned(), "bar".to_owned())
                    ]
                );
            }),
        );

        let client = TestClient::new(app);

        let res = client.get("/foo/bar").await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[crate::test]
    async fn type_that_uses_deserialize_any() {
        use time::Date;

        #[derive(Deserialize)]
        struct Params {
            a: Date,
            b: Date,
            c: Date,
        }

        let app = Router::new()
            .route(
                "/single/{a}",
                get(|Path(a): Path<Date>| async move { format!("single: {a}") }),
            )
            .route(
                "/tuple/{a}/{b}/{c}",
                get(|Path((a, b, c)): Path<(Date, Date, Date)>| async move {
                    format!("tuple: {a} {b} {c}")
                }),
            )
            .route(
                "/vec/{a}/{b}/{c}",
                get(|Path(vec): Path<Vec<Date>>| async move {
                    let [a, b, c]: [Date; 3] = vec.try_into().unwrap();
                    format!("vec: {a} {b} {c}")
                }),
            )
            .route(
                "/vec_pairs/{a}/{b}/{c}",
                get(|Path(vec): Path<Vec<(String, Date)>>| async move {
                    let [(_, a), (_, b), (_, c)]: [(String, Date); 3] = vec.try_into().unwrap();
                    format!("vec_pairs: {a} {b} {c}")
                }),
            )
            .route(
                "/map/{a}/{b}/{c}",
                get(|Path(mut map): Path<HashMap<String, Date>>| async move {
                    let a = map.remove("a").unwrap();
                    let b = map.remove("b").unwrap();
                    let c = map.remove("c").unwrap();
                    format!("map: {a} {b} {c}")
                }),
            )
            .route(
                "/struct/{a}/{b}/{c}",
                get(|Path(params): Path<Params>| async move {
                    format!("struct: {} {} {}", params.a, params.b, params.c)
                }),
            );

        let client = TestClient::new(app);

        let res = client.get("/single/2023-01-01").await;
        assert_eq!(res.text().await, "single: 2023-01-01");

        let res = client.get("/tuple/2023-01-01/2023-01-02/2023-01-03").await;
        assert_eq!(res.text().await, "tuple: 2023-01-01 2023-01-02 2023-01-03");

        let res = client.get("/vec/2023-01-01/2023-01-02/2023-01-03").await;
        assert_eq!(res.text().await, "vec: 2023-01-01 2023-01-02 2023-01-03");

        let res = client
            .get("/vec_pairs/2023-01-01/2023-01-02/2023-01-03")
            .await;
        assert_eq!(
            res.text().await,
            "vec_pairs: 2023-01-01 2023-01-02 2023-01-03",
        );

        let res = client.get("/map/2023-01-01/2023-01-02/2023-01-03").await;
        assert_eq!(res.text().await, "map: 2023-01-01 2023-01-02 2023-01-03");

        let res = client.get("/struct/2023-01-01/2023-01-02/2023-01-03").await;
        assert_eq!(res.text().await, "struct: 2023-01-01 2023-01-02 2023-01-03");
    }

    #[crate::test]
    async fn wrong_number_of_parameters_json() {
        use json_codec::Value;

        let app = Router::new()
            .route("/one/{a}", get(|_: Path<(Value, Value)>| async {}))
            .route("/two/{a}/{b}", get(|_: Path<Value>| async {}));

        let client = TestClient::new(app);

        let res = client.get("/one/1").await;
        assert!(res
            .text()
            .await
            .starts_with("Wrong number of path arguments for `Path`. Expected 2 but got 1"));

        let res = client.get("/two/1/2").await;
        assert!(res
            .text()
            .await
            .starts_with("Wrong number of path arguments for `Path`. Expected 1 but got 2"));
    }

    #[crate::test]
    async fn raw_path_params() {
        let app = Router::new().route(
            "/{a}/{b}/{c}",
            get(|params: RawPathParams| async move {
                params
                    .into_iter()
                    .map(|(key, value)| format!("{key}={value}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
        );

        let client = TestClient::new(app);
        let res = client.get("/foo/bar/baz").await;
        let body = res.text().await;
        assert_eq!(body, "a=foo b=bar c=baz");
    }

    #[crate::test]
    async fn deserialize_error_single_value() {
        let app = Router::new().route(
            "/resources/{res}",
            get(|res: Path<uuid::Uuid>| async move {
                let _res = res;
            }),
        );

        let client = TestClient::new(app);
        let response = client.get("/resources/123123-123-123123").await;
        let body = response.text().await;
        assert_eq!(
            body,
            "Invalid URL: Cannot parse `res` with value `123123-123-123123`: UUID parsing failed: invalid group count: expected 5, found 3"
        );
    }

    #[crate::test]
    async fn deserialize_error_multi_value() {
        let app = Router::new().route(
            "/resources/{res}/sub/{sub}",
            get(
                |Path((res, sub)): Path<(uuid::Uuid, uuid::Uuid)>| async move {
                    let _res = res;
                    let _sub = sub;
                },
            ),
        );

        let client = TestClient::new(app);
        let response = client.get("/resources/456456-123-456456/sub/123").await;
        let body = response.text().await;
        assert_eq!(
            body,
            "Invalid URL: Cannot parse `res` with value `456456-123-456456`: UUID parsing failed: invalid group count: expected 5, found 3"
        );
    }

    #[crate::test]
    async fn regression_3038() {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct MoreChars {
            first_two: [char; 2],
            second_two: [char; 2],
            crate_name: String,
        }

        let app = Router::new().route(
            "/{first_two}/{second_two}/{crate_name}",
            get(|Path(_): Path<MoreChars>| async move {}),
        );

        let client = TestClient::new(app);
        let res = client.get("/te/st/_thing").await;
        let body = res.text().await;
        assert_eq!(body, "Invalid URL: array types are not supported");
    }
}
