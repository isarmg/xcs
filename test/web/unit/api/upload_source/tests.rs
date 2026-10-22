use super::*;
use std::{
    cell::Cell,
    future::Future,
    task::{Context, Poll, Waker},
};

fn run<T>(future: impl Future<Output = T>) -> T {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("unexpected pending future"),
    }
}

#[test]
fn browser_preflight_requires_exact_nonnegative_integers() {
    for size in [
        0.0,
        64.0 * 1024.0 * 1024.0 + 1.0,
        MAX_EXACT_BROWSER_SIZE as f64,
    ] {
        assert!(browser_size(size).is_ok());
    }
    for size in [f64::NAN, f64::INFINITY, -1.0, 0.5, (1u64 << 53) as f64] {
        assert!(browser_size(size).is_err());
    }
}

#[test]
fn server_limit_is_lossless_and_fail_closed() {
    for limit in [0, 1, 1 << 30, u64::MAX] {
        let value =
            json_codec::json!({"complete_upload":true,"max_upload_bytes":limit.to_string()});
        assert_eq!(capability_limit(&value).unwrap(), limit);
        assert!(validate_limit(limit, limit).is_ok());
        if limit < u64::MAX {
            assert!(validate_limit(limit + 1, limit).is_err());
        }
    }
    for value in [
        json_codec::json!({}),
        json_codec::json!({"complete_upload":true,"max_upload_bytes":123}),
        json_codec::json!({"complete_upload":true,"max_upload_bytes":"18446744073709551616"}),
        json_codec::json!({"complete_upload":false,"max_upload_bytes":"1"}),
    ] {
        assert!(capability_limit(&value).is_err());
    }
}

#[test]
fn incremental_hash_matches_empty_exact_and_partial_chunks() {
    for size in [
        0,
        1,
        HASH_CHUNK_BYTES - 1,
        HASH_CHUNK_BYTES,
        HASH_CHUNK_BYTES + 1,
        HASH_CHUNK_BYTES * 3 + 17,
    ] {
        let content = vec![0x5a; size as usize];
        let calls = Cell::new(0);
        let actual = run(hash_chunks(
            size,
            || Ok(()),
            |start, end| {
                assert!(end - start <= HASH_CHUNK_BYTES);
                calls.set(calls.get() + 1);
                std::future::ready(Ok(content[start as usize..end as usize].to_vec()))
            },
        ))
        .unwrap();
        assert_eq!(actual, blake3::hash(&content).to_hex().to_string());
        assert_eq!(calls.get(), size.div_ceil(HASH_CHUNK_BYTES));
    }
}

#[test]
fn session_change_after_read_stops_before_next_chunk_even_on_read_failure() {
    for failed in [false, true] {
        let changed = Cell::new(false);
        let calls = Cell::new(0);
        let result = run(hash_chunks(
            HASH_CHUNK_BYTES + 1,
            || {
                if changed.get() {
                    Err(super::super::session_changed())
                } else {
                    Ok(())
                }
            },
            |start, end| {
                calls.set(calls.get() + 1);
                changed.set(true);
                std::future::ready(if failed {
                    Err(invalid_response())
                } else {
                    Ok(vec![0; (end - start) as usize])
                })
            },
        ));
        assert_eq!(result.unwrap_err().code, "SESSION_CHANGED");
        assert_eq!(calls.get(), 1);
    }
}

#[test]
fn invalid_chunk_lengths_fail_closed() {
    for length in [0, 2] {
        assert!(
            run(hash_chunks(
                1,
                || Ok(()),
                |_, _| std::future::ready(Ok(vec![0; length]))
            ))
            .is_err()
        );
    }
}

#[test]
fn dropping_pending_hash_cancels_without_another_read() {
    let calls = Cell::new(0);
    let mut future = Box::pin(hash_chunks(
        HASH_CHUNK_BYTES + 1,
        || Ok(()),
        |_, _| {
            calls.set(calls.get() + 1);
            std::future::pending::<Result<Vec<u8>, Error>>()
        },
    ));
    let waker = Waker::noop();
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(waker))
            .is_pending()
    );
    drop(future);
    assert_eq!(calls.get(), 1);
}

#[test]
fn stale_admission_never_starts_a_read() {
    let result = run(hash_chunks(
        1,
        || Err(super::super::session_changed()),
        |_, _| {
            panic!("a stale session must not read the file");
            #[allow(unreachable_code)]
            std::future::ready(Ok(Vec::new()))
        },
    ));
    assert_eq!(result.unwrap_err().code, "SESSION_CHANGED");
}

#[test]
fn read_failure_does_not_advance_or_retry() {
    let calls = Cell::new(0);
    let result = run(hash_chunks(
        HASH_CHUNK_BYTES + 1,
        || Ok(()),
        |_, _| {
            calls.set(calls.get() + 1);
            std::future::ready(Err(error("READ_FAILED", "read failed")))
        },
    ));
    assert_eq!(result.unwrap_err().code, "READ_FAILED");
    assert_eq!(calls.get(), 1);
}

#[test]
fn session_fence_preserves_rejection_status_and_request_id() {
    let admission = SessionAdmission::capture();
    super::super::clear_session();
    let mut rejection = error("UNAUTHORIZED", "expired").with_status(401);
    rejection.request_id = Some("request-1".into());
    let result: Result<(), Error> = Err(rejection);
    let stopped = check_completion(admission, &result).unwrap_err();
    assert_eq!(stopped.code, "SESSION_CHANGED");
    assert_eq!(stopped.status, 401);
    assert_eq!(stopped.request_id.as_deref(), Some("request-1"));
}

fn capabilities(idle: &str, total: &str) -> json_codec::Value {
    json_codec::json!({"complete_upload":true,"max_upload_bytes":"1073741824",
        "transfer_idle_timeout_ms":idle,"transfer_total_timeout_ms":total})
}

#[test]
fn advertised_transfer_timeouts_preserve_defaults_subseconds_and_maximum() {
    let defaults = parse_transfer_limits(&capabilities("60000", "3600000")).unwrap();
    assert_eq!(defaults.idle_timeout_ms, 60_000);
    assert_eq!(defaults.total_timeout_ms, 3_600_000);
    for (idle, total) in [("1", "1"), ("37", "1500"), ("604800000", "604800000")] {
        let limits = parse_transfer_limits(&capabilities(idle, total)).unwrap();
        assert_eq!(limits.idle_timeout_ms.to_string(), idle);
        assert_eq!(limits.total_timeout_ms.to_string(), total);
        assert!(limits.total_timeout_ms <= i32::MAX as u32);
    }
}

#[test]
fn advertised_timeouts_reject_missing_malformed_zero_overflow_and_reversed_values() {
    for field in ["transfer_idle_timeout_ms", "transfer_total_timeout_ms"] {
        for bad in [
            json_codec::Value::Null,
            json_codec::json!(60000),
            json_codec::json!(true),
            json_codec::json!(""),
            json_codec::json!("0"),
            json_codec::json!("-1"),
            json_codec::json!("0.5"),
            json_codec::json!("604800001"),
            json_codec::json!("2147483648"),
            json_codec::json!("4294967296"),
            json_codec::json!("18446744073709551616"),
        ] {
            let mut value = capabilities("60000", "3600000");
            value[field] = bad;
            assert_eq!(
                parse_transfer_limits(&value).unwrap_err().code,
                "INVALID_TRANSFER_LIMITS"
            );
        }
        let mut value = capabilities("60000", "3600000");
        value.as_object_mut().unwrap().remove(field);
        assert_eq!(
            parse_transfer_limits(&value).unwrap_err().code,
            "INVALID_TRANSFER_LIMITS"
        );
    }
    assert!(parse_transfer_limits(&capabilities("1001", "1000")).is_err());
}

#[test]
fn download_timeout_parsing_does_not_require_upload_support() {
    let mut value = capabilities("60000", "3600000");
    value["complete_upload"] = json_codec::json!(false);
    value.as_object_mut().unwrap().remove("max_upload_bytes");
    assert!(parse_transfer_limits(&value).is_ok());
    assert!(capability_limit(&value).is_err());
}
