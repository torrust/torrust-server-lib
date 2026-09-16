//! This module contains functions to handle signals.
use derive_more::Display;
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

/// This is the message that the "launcher" spawned task receives from the main
/// application process to notify the service to shutdown.
///
#[derive(Copy, Clone, Debug, Display)]
pub enum Halted {
    Normal,
}

/// Resolves when the injected cancellation token is cancelled.
///
/// Unlike [`global_shutdown_signal`], this function never subscribes to an
/// operating-system signal. The caller owns the cancellation lifecycle.
#[instrument(skip(cancellation_token))]
pub async fn cancellation_signal(cancellation_token: CancellationToken) {
    cancellation_token.cancelled().await;
}

/// Resolves on `ctrl_c` or the `terminate` signal.
///
/// # Panics
///
/// Will panic if the `ctrl_c` or `terminate` signal resolves with an error.
#[instrument(skip())]
pub async fn global_shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {tracing::warn!("caught interrupt signal (ctrl-c), halting...");},
        () = terminate => {tracing::warn!("caught interrupt signal (terminate), halting...");}
    }
}

/// Resolves when the `stop_receiver` or the `global_shutdown_signal()` resolves.
///
/// # Panics
///
/// Will panic if the `stop_receiver` resolves with an error.
#[instrument(skip(rx_halt))]
pub async fn shutdown_signal(rx_halt: tokio::sync::oneshot::Receiver<Halted>) {
    let halt = async {
        match rx_halt.await {
            Ok(signal) => signal,
            Err(err) => panic!("Failed to install stop signal: {err}"),
        }
    };

    tokio::select! {
        signal = halt => { tracing::debug!("Halt signal processed: {}", signal) },
        () = global_shutdown_signal() => { tracing::debug!("Global shutdown signal processed") }
    }
}

/// Same as `shutdown_signal()`, but shows a message when it resolves.
#[instrument(skip(rx_halt))]
pub async fn shutdown_signal_with_message(rx_halt: tokio::sync::oneshot::Receiver<Halted>, message: String) {
    shutdown_signal(rx_halt).await;

    tracing::info!("{message}");
}

#[cfg(test)]
mod tests {
    use tokio::sync::oneshot;
    use tokio_util::sync::CancellationToken;

    use super::{Halted, cancellation_signal, shutdown_signal};

    #[tokio::test]
    async fn cancellation_signal_should_resolve_when_the_token_is_cancelled() {
        let cancellation_token = CancellationToken::new();
        let wait_task = tokio::spawn(cancellation_signal(cancellation_token.clone()));

        tokio::task::yield_now().await;

        assert!(!wait_task.is_finished());

        cancellation_token.cancel();

        wait_task.await.expect("cancellation wait task should complete");
    }

    #[tokio::test]
    async fn shutdown_signal_should_resolve_when_it_receives_a_legacy_halted_message() {
        let (halt_sender, halt_receiver) = oneshot::channel();
        let wait_task = tokio::spawn(shutdown_signal(halt_receiver));

        halt_sender
            .send(Halted::Normal)
            .expect("legacy halt receiver should still be waiting");

        wait_task.await.expect("legacy shutdown wait task should complete");
    }
}
