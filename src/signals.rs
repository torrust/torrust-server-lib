//! This module contains functions to handle signals.
use tokio_util::sync::CancellationToken;
use torrust_net_primitives::service_binding::ServiceBinding;
use tracing::instrument;

/// This is the message that the "launcher" spawned task sends to the main
/// application process to notify the service was successfully started.
///
#[derive(Debug)]
pub struct Started {
    pub service_binding: ServiceBinding,
    pub address: std::net::SocketAddr,
}

/// Resolves when the injected cancellation token is cancelled.
///
/// This function never subscribes to an operating-system signal. The caller
/// owns the cancellation lifecycle.
#[instrument(skip(cancellation_token))]
pub async fn cancellation_signal(cancellation_token: CancellationToken) {
    cancellation_token.cancelled().await;
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;

    use super::cancellation_signal;

    #[tokio::test]
    async fn cancellation_signal_should_resolve_when_the_token_is_cancelled() {
        let cancellation_token = CancellationToken::new();
        let wait_task = tokio::spawn(cancellation_signal(cancellation_token.clone()));

        tokio::task::yield_now().await;

        assert!(!wait_task.is_finished());

        cancellation_token.cancel();

        wait_task.await.expect("cancellation wait task should complete");
    }
}
