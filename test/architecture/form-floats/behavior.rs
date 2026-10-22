use http_body::BodyExt;
use http_routing::{
    body::Body,
    extract::{FromRequest, Query},
    http::Request,
    response::IntoResponse,
    Form,
};
use std::{
    collections::BTreeMap,
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};
struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}
fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    match Box::pin(future)
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("in-memory form extraction unexpectedly pending"),
    }
}
fn roundtrip32(bits: u32) {
    let value = f32::from_bits(bits);
    let wire = form_codec::to_string(&[("x", value)]).unwrap();
    assert!(!wire.contains('+'), "literal plus would decode as a space");
    let parsed: Vec<(String, f32)> = form_codec::from_str(&wire).unwrap();
    if value.is_nan() {
        assert!(parsed[0].1.is_nan());
    } else {
        assert_eq!(parsed[0].1.to_bits(), bits, "{wire}");
    }
}
fn roundtrip64(bits: u64) {
    let value = f64::from_bits(bits);
    let wire = form_codec::to_string(&[("x", value)]).unwrap();
    assert!(!wire.contains('+'), "literal plus would decode as a space");
    let parsed: Vec<(String, f64)> = form_codec::from_str(&wire).unwrap();
    if value.is_nan() {
        assert!(parsed[0].1.is_nan());
    } else {
        assert_eq!(parsed[0].1.to_bits(), bits, "{wire}");
    }
}
#[test]
fn canonical_wire_uses_zmij_positive_exponent_sign() {
    // Intentional canonical spelling change: the retired Ryu spelling omitted '+'.
    assert_eq!(
        form_codec::to_string(&[("x", 1e16f64)]).unwrap(),
        "x=1e%2B16"
    );
    assert_eq!(
        form_codec::to_string(&[("x", f32::MAX)]).unwrap(),
        "x=3.4028235e%2B38"
    );
    for (value, expected) in [
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (f64::INFINITY, "inf"),
        (f64::NEG_INFINITY, "-inf"),
        (f64::NAN, "NaN"),
    ] {
        assert_eq!(
            form_codec::to_string(&[("x", value)]).unwrap(),
            format!("x={expected}")
        );
    }
}
#[test]
fn binary_and_decimal_boundaries_preserve_float_bits() {
    for sign in [0, 1u32 << 31] {
        for exp in 0..=255u32 {
            for m in [0, 1, 2, 3, 0x3fffff, 0x400000, 0x400001, 0x7ffffe, 0x7fffff] {
                roundtrip32(sign | (exp << 23) | m);
            }
        }
    }
    for sign in [0, 1u64 << 63] {
        for exp in 0..=2047u64 {
            for m in [
                0,
                1,
                2,
                3,
                0x7ffffffffffff,
                0x8000000000000,
                0x8000000000001,
                0xffffffffffffe,
                0xfffffffffffff,
            ] {
                roundtrip64(sign | (exp << 52) | m);
            }
        }
    }
    for exp in -45..=38 {
        let bits = 10f32.powi(exp).to_bits();
        for delta in -8i64..=8 {
            let bits = (bits as i64 + delta).clamp(0, u32::MAX as i64) as u32;
            roundtrip32(bits);
            roundtrip32(bits | (1 << 31));
        }
    }
    for exp in -323..=308 {
        let bits = 10f64.powi(exp).to_bits();
        for delta in -8i128..=8 {
            let bits = (bits as i128 + delta).clamp(0, u64::MAX as i128) as u64;
            roundtrip64(bits);
            roundtrip64(bits | (1 << 63));
        }
    }
}
#[test]
fn deterministic_random_bits_roundtrip() {
    let mut rng = 0x19a24309a92f1c79u64;
    for _ in 0..100_000 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        roundtrip64(rng);
        roundtrip32((rng >> 32) as u32);
    }
}
#[test]
fn routing_form_response_and_query_decode_encoded_plus() {
    for value in [1e16, f64::MAX, -0.0, f64::from_bits(1), f64::MIN_POSITIVE] {
        let response = Form([("x", value)]).into_response();
        assert_eq!(
            response.headers()["content-type"],
            "application/x-www-form-urlencoded"
        );
        let body = run(response.into_body().collect()).unwrap().to_bytes();
        let wire = std::str::from_utf8(&body).unwrap();
        assert!(!wire.contains('+'));
        let uri = format!("http://example.test/?{wire}").parse().unwrap();
        let query = Query::<BTreeMap<String, f64>>::try_from_uri(&uri).unwrap();
        assert_eq!(query.0["x"].to_bits(), value.to_bits());
        let request = Request::builder()
            .method("POST")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();
        let form = run(Form::<BTreeMap<String, f64>>::from_request(request, &())).unwrap();
        assert_eq!(form.0["x"].to_bits(), value.to_bits());
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn reqwest_form_and_query_use_encoded_plus() {
    let client = reqwest::Client::new();
    for value in [1e16, f64::MAX, -0.0, f64::from_bits(1), f64::MIN_POSITIVE] {
        let request = client
            .post("http://example.test/")
            .form(&[("x", value)])
            .build()
            .unwrap();
        assert_eq!(
            request.headers()["content-type"],
            "application/x-www-form-urlencoded"
        );
        {
            let body = request.body().unwrap().as_bytes().unwrap();
            assert_eq!(
                body,
                form_codec::to_string(&[("x", value)]).unwrap().as_bytes()
            );
            let request = Request::builder()
                .method("POST")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(body.to_vec()))
                .unwrap();
            let form = run(Form::<BTreeMap<String, f64>>::from_request(request, &())).unwrap();
            assert_eq!(form.0["x"].to_bits(), value.to_bits());
        }
        let request = client
            .get("http://example.test/")
            .query(&[("x", value)])
            .build()
            .unwrap();
        let uri = request.url().as_str().parse().unwrap();
        let query = Query::<BTreeMap<String, f64>>::try_from_uri(&uri).unwrap();
        assert_eq!(query.0["x"].to_bits(), value.to_bits());
    }
}
