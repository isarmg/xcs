#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! classify_grpc_metadata_test {
        (
            name: $name:ident,
            status: $status:expr,
            success_flags: $success_flags:expr,
            expected: $expected:expr,
        ) => {
            #[test]
            fn $name() {
                let mut headers = HeaderMap::new();
                headers.insert("grpc-status", $status.parse().unwrap());
                let status = classify_grpc_metadata(&headers, $success_flags);
                assert_eq!(status, $expected);
            }
        };
    }

    classify_grpc_metadata_test! {
        name: basic_ok,
        status: "0",
        success_flags: GrpcCodeBitmask::OK,
        expected: ParsedGrpcStatus::Success,
    }

    classify_grpc_metadata_test! {
        name: basic_error,
        status: "1",
        success_flags: GrpcCodeBitmask::OK,
        expected: ParsedGrpcStatus::NonSuccess(NonZeroI32::new(1).unwrap()),
    }

    classify_grpc_metadata_test! {
        name: two_success_codes_first_matches,
        status: "0",
        success_flags: GrpcCodeBitmask::OK | GrpcCodeBitmask::INVALID_ARGUMENT,
        expected: ParsedGrpcStatus::Success,
    }

    classify_grpc_metadata_test! {
        name: two_success_codes_second_matches,
        status: "3",
        success_flags: GrpcCodeBitmask::OK | GrpcCodeBitmask::INVALID_ARGUMENT,
        expected: ParsedGrpcStatus::Success,
    }

    classify_grpc_metadata_test! {
        name: two_success_codes_none_matches,
        status: "16",
        success_flags: GrpcCodeBitmask::OK | GrpcCodeBitmask::INVALID_ARGUMENT,
        expected: ParsedGrpcStatus::NonSuccess(NonZeroI32::new(16).unwrap()),
    }

    #[test]
    fn grpc_code_from_i32_known_codes() {
        assert!(matches!(GrpcCode::from(0), GrpcCode::Ok));
        assert!(matches!(GrpcCode::from(1), GrpcCode::Cancelled));
        assert!(matches!(GrpcCode::from(4), GrpcCode::DeadlineExceeded));
        assert!(matches!(GrpcCode::from(13), GrpcCode::Internal));
        assert!(matches!(GrpcCode::from(16), GrpcCode::Unauthenticated));
    }

    #[test]
    fn grpc_code_from_i32_unknown_codes() {
        assert!(matches!(GrpcCode::from(17), GrpcCode::Unknown));
        assert!(matches!(GrpcCode::from(-1), GrpcCode::Unknown));
        assert!(matches!(GrpcCode::from(9999), GrpcCode::Unknown));
    }

    #[test]
    fn grpc_code_from_non_zero_i32() {
        let code = NonZeroI32::new(7).unwrap();
        assert!(matches!(GrpcCode::from(code), GrpcCode::PermissionDenied));

        let code = NonZeroI32::new(99).unwrap();
        assert!(matches!(GrpcCode::from(code), GrpcCode::Unknown));
    }
}
