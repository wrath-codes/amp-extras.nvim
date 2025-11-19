//! Global Async Runtime
//!
//! Provides a shared Tokio runtime for the entire plugin.
//! Used by both the WebSocket server and async commands.

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

/// Global shared Tokio runtime
///
/// This runtime is initialized lazily on first use. It is used for:
/// - Async commands (short-lived CLI processes)
/// - Background tasks
pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create Tokio runtime")
});

/// Spawn a future on the global runtime
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    RUNTIME.spawn(future)
}

/// Run a future to completion (blocking the current thread)
///
/// useful for initializing resources that must be ready before proceeding
pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
    RUNTIME.block_on(future)
}
