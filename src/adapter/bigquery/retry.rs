//! BigQuery Retry Logic and Error Classification
//!
//! リトライロジックとエラー分類

// Retry configuration based on Google Cloud best practices
// See: https://cloud.google.com/bigquery/docs/streaming-data-into-bigquery
pub const MAX_RETRIES: u32 = 5;
pub const MAX_CONNECTION_RESETS: u32 = 3; // Max number of times to recreate client
pub const INITIAL_RETRY_DELAY_MS: u64 = 1000; // 1 second (Google recommends starting small)
pub const MAX_RETRY_DELAY_MS: u64 = 32000; // 32 seconds max
pub const BATCH_DELAY_MS: u64 = 200; // 200ms between batches to avoid rate limits

/// Calculate retry delay with exponential backoff
pub fn calculate_retry_delay(retry_count: u32) -> u64 {
    std::cmp::min(
        INITIAL_RETRY_DELAY_MS * (1 << (retry_count - 1)),
        MAX_RETRY_DELAY_MS,
    )
}

/// Convert error chain to string including all causes
pub fn error_chain_to_string(e: &anyhow::Error) -> String {
    let mut messages = Vec::new();
    for cause in e.chain() {
        messages.push(cause.to_string());
    }
    messages.join(" | ")
}

/// Check if an error requires connection reset (new client creation)
pub fn is_connection_error(error_msg: &str) -> bool {
    error_msg.contains("Broken pipe")
        || error_msg.contains("broken pipe")
        || error_msg.contains("Connection reset")
        || error_msg.contains("connection reset")
        || error_msg.contains("Connection refused")
        || error_msg.contains("connection refused")
        || error_msg.contains("connection error")
        || error_msg.contains("EOF")
        || error_msg.contains("unexpected end of file")
}

/// Check if an error is transient (can retry with same client)
pub fn is_transient_error(error_msg: &str) -> bool {
    error_msg.contains("not found")
        || error_msg.contains("deleted")
        || error_msg.contains("503")
        || error_msg.contains("500")
        || error_msg.contains("403")
        || error_msg.contains("429")
        || error_msg.contains("rate")
        || error_msg.contains("quota")
        || error_msg.contains("Quota")
        || error_msg.contains("timeout")
        || error_msg.contains("Timeout")
}

/// Check if an error message indicates a retryable error
pub fn is_retryable_error(error_msg: &str) -> bool {
    is_connection_error(error_msg) || is_transient_error(error_msg)
}

/// Check if an error indicates the request was too large (413)
pub fn is_request_too_large_error(error_msg: &str) -> bool {
    error_msg.contains("413") || error_msg.contains("Request Entity Too Large")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_retry_delay_first_retry() {
        let delay = calculate_retry_delay(1);
        assert_eq!(delay, INITIAL_RETRY_DELAY_MS); // 1000ms
    }

    #[test]
    fn test_calculate_retry_delay_second_retry() {
        let delay = calculate_retry_delay(2);
        assert_eq!(delay, INITIAL_RETRY_DELAY_MS * 2); // 2000ms
    }

    #[test]
    fn test_calculate_retry_delay_third_retry() {
        let delay = calculate_retry_delay(3);
        assert_eq!(delay, INITIAL_RETRY_DELAY_MS * 4); // 4000ms
    }

    #[test]
    fn test_calculate_retry_delay_capped() {
        // Very high retry count should be capped at MAX_RETRY_DELAY_MS
        let delay = calculate_retry_delay(10);
        assert_eq!(delay, MAX_RETRY_DELAY_MS);
    }

    #[test]
    fn test_is_connection_error() {
        // Test broken pipe variations
        assert!(is_connection_error("Broken pipe"));
        assert!(is_connection_error("broken pipe (os error 32)"));
        assert!(is_connection_error("error sending request: Broken pipe"));

        // Test connection reset
        assert!(is_connection_error("Connection reset"));
        assert!(is_connection_error("connection reset by peer"));
        assert!(is_connection_error("Connection reset by peer"));

        // Test connection refused
        assert!(is_connection_error("Connection refused"));

        // Test EOF
        assert!(is_connection_error("EOF"));
        assert!(is_connection_error("unexpected end of file"));

        // Test non-connection errors
        assert!(!is_connection_error("503 Service Unavailable"));
        assert!(!is_connection_error("429 Too Many Requests"));
        assert!(!is_connection_error("Table not found"));
    }

    #[test]
    fn test_is_transient_error() {
        // Test table not found
        assert!(is_transient_error("Table not found"));
        assert!(is_transient_error("Resource was deleted"));

        // Test server errors
        assert!(is_transient_error("503 Service Unavailable"));
        assert!(is_transient_error("500 Internal Server Error"));

        // Test rate limiting
        assert!(is_transient_error("403 Quota exceeded"));
        assert!(is_transient_error("429 Too Many Requests"));
        assert!(is_transient_error("rate limit exceeded"));
        assert!(is_transient_error("quota exceeded"));
        assert!(is_transient_error("Quota limit reached"));

        // Test timeout
        assert!(is_transient_error("timeout"));
        assert!(is_transient_error("Timeout waiting for response"));

        // Test non-transient errors
        assert!(!is_transient_error("Authentication failed"));
        assert!(!is_transient_error("Invalid request"));
        assert!(!is_transient_error("Broken pipe"));
    }

    #[test]
    fn test_is_retryable_error_network_errors() {
        assert!(is_retryable_error("connection error"));
        assert!(is_retryable_error("Connection refused"));
        assert!(is_retryable_error("Broken pipe"));
        assert!(is_retryable_error("broken pipe (os error 32)"));
        assert!(is_retryable_error("timeout"));
        assert!(is_retryable_error("Timeout waiting for response"));
        assert!(is_retryable_error("connection reset by peer"));
    }

    #[test]
    fn test_is_retryable_error_server_errors() {
        assert!(is_retryable_error("503 Service Unavailable"));
        assert!(is_retryable_error("500 Internal Server Error"));
    }

    #[test]
    fn test_is_retryable_error_not_found() {
        assert!(is_retryable_error("Table not found"));
        assert!(is_retryable_error("Resource was deleted"));
    }

    #[test]
    fn test_is_retryable_error_rate_limit() {
        assert!(is_retryable_error("403 Quota exceeded"));
        assert!(is_retryable_error("429 Too Many Requests"));
        assert!(is_retryable_error("rate limit exceeded"));
        assert!(is_retryable_error("quota exceeded"));
        assert!(is_retryable_error("Quota limit reached"));
    }

    #[test]
    fn test_is_retryable_error_non_retryable() {
        assert!(!is_retryable_error("Invalid request"));
        assert!(!is_retryable_error("Authentication failed"));
        assert!(!is_retryable_error("Bad request syntax"));
    }

    #[test]
    fn test_is_request_too_large_error() {
        assert!(is_request_too_large_error("413 Request Entity Too Large"));
        assert!(is_request_too_large_error(
            "HTTP status client error (413 Request Entity Too Large)"
        ));
        assert!(is_request_too_large_error("error 413"));
        assert!(!is_request_too_large_error("500 Internal Server Error"));
        assert!(!is_request_too_large_error("Connection refused"));
    }

    #[test]
    fn test_error_classification_disjoint() {
        // Connection errors and transient errors should be disjoint sets
        let connection_errors = vec![
            "Broken pipe",
            "Connection reset",
            "Connection refused",
            "EOF",
        ];

        let transient_errors = vec![
            "503 Service Unavailable",
            "429 Too Many Requests",
            "Table not found",
            "timeout",
        ];

        for err in &connection_errors {
            assert!(
                is_connection_error(err),
                "{} should be a connection error",
                err
            );
            assert!(
                !is_transient_error(err),
                "{} should not be a transient error",
                err
            );
        }

        for err in &transient_errors {
            assert!(
                is_transient_error(err),
                "{} should be a transient error",
                err
            );
            assert!(
                !is_connection_error(err),
                "{} should not be a connection error",
                err
            );
        }
    }

    #[test]
    fn test_error_chain_to_string() {
        use anyhow::Context;

        // Create nested error
        let inner_error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Broken pipe");
        let error = anyhow::Error::from(inner_error)
            .context("client error")
            .context("BigQuery insert failed");

        let error_msg = error_chain_to_string(&error);

        // Should contain all parts of the chain
        assert!(error_msg.contains("BigQuery insert failed"));
        assert!(error_msg.contains("Broken pipe"));
        assert!(is_retryable_error(&error_msg));
    }

    #[test]
    fn test_constants() {
        // Verify constants are set to expected values
        assert_eq!(MAX_RETRIES, 5);
        assert_eq!(INITIAL_RETRY_DELAY_MS, 1000);
        assert_eq!(MAX_RETRY_DELAY_MS, 32000);
        assert_eq!(BATCH_DELAY_MS, 200);
    }
}
