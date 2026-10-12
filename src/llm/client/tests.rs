use super::is_retryable_status;

#[test]
fn rate_limit_and_server_errors_are_retryable() {
    for code in [408, 409, 425, 429, 500, 502, 503, 504, 599] {
        assert!(is_retryable_status(code), "{code} should be retryable");
    }
}

#[test]
fn client_errors_are_not_retryable() {
    for code in [400, 401, 403, 404, 422] {
        assert!(!is_retryable_status(code), "{code} should not be retryable");
    }
}
