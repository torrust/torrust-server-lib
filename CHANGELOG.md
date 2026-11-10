# Changelog

## 0.6.0

### Removed

The legacy shutdown API in `torrust_server_lib::signals`, deprecated since `0.5.0`, is removed.
This is a breaking release.

| Removed                         | Migrate to                                                                                                           |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `Halted`                        | Inject a `tokio_util::sync::CancellationToken`.                                                                      |
| `shutdown_signal`               | `cancellation_signal(token)`.                                                                                        |
| `shutdown_signal_with_message`  | `cancellation_signal(token)` followed by your own log line.                                                          |
| `global_shutdown_signal`        | The executable subscribes to SIGINT/SIGTERM and cancels the injected token.                                          |

`Started` and `cancellation_signal` are unchanged.

The crate no longer enables the `tokio` `signal` feature and no longer depends on `derive_more`.

For the migration steps, see the `0.5.0` section below: executables own SIGINT/SIGTERM and cancel an
injected `CancellationToken`.

## 0.5.0

### Deprecated

The legacy shutdown API in `torrust_server_lib::signals` is deprecated. There are no behaviour
changes and nothing is removed in this release; using these symbols now emits a compiler warning.

| Deprecated                      | Migrate to                                                                                                                 |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `Halted`                        | Inject a `tokio_util::sync::CancellationToken`.                                                                            |
| `shutdown_signal`               | `cancellation_signal(token)`.                                                                                              |
| `shutdown_signal_with_message`  | `cancellation_signal(token)` followed by your own log line.                                                                |
| `global_shutdown_signal`        | Libraries must not subscribe to signals. The executable subscribes to SIGINT/SIGTERM and cancels the injected token.       |

`Started` and `cancellation_signal` are not deprecated.

### Migration

1. Replace the `oneshot::Sender<Halted>` / `Receiver<Halted>` pair with a `CancellationToken`
   shared between the code that requests shutdown and the launcher task.
2. In the launcher, await `cancellation_signal(token.clone())` where you awaited
   `shutdown_signal(rx_halt)`; log the message afterwards instead of using `shutdown_signal_with_message`.
3. In the executable, subscribe to SIGINT/SIGTERM (for example `tokio::signal`) and call
   `token.cancel()`, instead of relying on `global_shutdown_signal`.

### Support window

The deprecated items remain available, unchanged, for all `0.5.x` releases. They will be removed
in `0.6.0`, a later breaking release.
