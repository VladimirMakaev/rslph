use tokio::signal;
use tokio_util::sync::CancellationToken;

/// Set up Ctrl+C handler that cancels the token when signal received.
/// Returns the token for use in execution loops.
///
/// The handler spawns a background task that waits for Ctrl+C.
/// When received, it cancels the token, allowing graceful shutdown.
pub fn setup_ctrl_c_handler() -> CancellationToken {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        // Signal received - cancel the token
        token_clone.cancel();
    });

    token
}

/// Check if cancellation was requested.
/// Use this for quick checks in loops.
pub fn is_cancelled(token: &CancellationToken) -> bool {
    token.is_cancelled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_ctrl_c_handler_returns_token() {
        let token = setup_ctrl_c_handler();
        assert!(!token.is_cancelled());
    }

    #[tokio::test]
    async fn test_is_cancelled_utility() {
        let token = CancellationToken::new();
        assert!(!is_cancelled(&token));
        token.cancel();
        assert!(is_cancelled(&token));
    }
}
