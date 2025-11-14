//! WebSocket server for Amp CLI integration
//!
//! This module implements a WebSocket server that listens on a random port,
//! writes lockfiles with auth tokens, and accepts connections from Amp CLI.

mod connection;
mod events;
mod hub;
mod ws_server;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
};

pub use hub::Hub;
use once_cell::sync::Lazy;

/// Server handle for lifecycle management
pub struct ServerHandle {
    shutdown:      Arc<AtomicBool>,
    join_handle:   Option<JoinHandle<()>>,
    lockfile_path: PathBuf,
    hub:           Hub,
    port:          u16,
}

impl ServerHandle {
    /// Stop the server and clean up resources
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);

        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }

        let _ = std::fs::remove_file(&self.lockfile_path);
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Global server instance
static SERVER: Lazy<Mutex<Option<ServerHandle>>> = Lazy::new(|| Mutex::new(None));

/// Start the WebSocket server
///
/// Returns (port, token, lockfile_path)
pub fn start() -> crate::errors::Result<(u16, String, PathBuf)> {
    // Initialize AsyncHandle for IDE operations (nvim/notify)
    crate::ide_ops::init_async_handle()?;
    let mut server = SERVER.lock().unwrap();

    if server.is_some() {
        return Err(crate::errors::AmpError::Other(
            "Server already running".into(),
        ));
    }

    // Bind to random port
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();

    // Generate token and write lockfile
    let token = crate::lockfile::generate_token(32);
    let lockfile_path = crate::lockfile::write_lockfile(port, &token)?;

    // Create hub for client management
    let hub = Hub::new();

    // Spawn server thread
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    let token_clone = token.clone();
    let hub_clone = hub.clone();

    let join_handle = std::thread::spawn(move || {
        ws_server::run_accept_loop(listener, token_clone, hub_clone, shutdown_clone);
    });

    // Store server handle
    let handle = ServerHandle {
        shutdown,
        join_handle: Some(join_handle),
        lockfile_path: lockfile_path.clone(),
        hub,
        port,
    };

    *server = Some(handle);

    // Fire server started event
    #[cfg(not(test))]
    events::notify_server_started();

    Ok((port, token, lockfile_path))
}

/// Stop the WebSocket server
pub fn stop() {
    let mut server = SERVER.lock().unwrap();

    if let Some(mut handle) = server.take() {
        handle.stop();

        // Fire server stopped event
        #[cfg(not(test))]
        events::notify_server_stopped();
    }
}

/// Check if server is running
pub fn is_running() -> bool {
    let server = SERVER.lock().unwrap();

    server
        .as_ref()
        .map(|h| !h.shutdown.load(Ordering::Relaxed))
        .unwrap_or(false)
}

/// Get the Hub instance (if server is running)
///
/// Returns None if server is not running.
pub fn get_hub() -> Option<Hub> {
    let server = SERVER.lock().unwrap();

    server
        .as_ref()
        .filter(|h| !h.shutdown.load(Ordering::Relaxed))
        .map(|h| h.hub.clone())
}

/// Get the server port (if server is running)
///
/// Returns None if server is not running.
pub fn get_port() -> Option<u16> {
    let server = SERVER.lock().unwrap();

    server
        .as_ref()
        .filter(|h| !h.shutdown.load(Ordering::Relaxed))
        .map(|h| h.port)
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;

    // NOTE: Server lifecycle tests require AsyncHandle from Neovim context.
    // These tests are skipped in Rust unit tests.
    // Run integration tests with: just test-integration

    #[test]
    #[ignore = "Requires Neovim context - run 'just test-integration'"]
    fn test_server_start_and_stop() {
        // This test is covered by tests/server_test.lua
    }

    #[test]
    #[ignore = "Requires Neovim context - run 'just test-integration'"]
    fn test_server_cannot_start_twice() {
        // This test is covered by tests/server_test.lua
    }

    #[test]
    fn test_stop_without_start() {
        // Should not panic when stopping non-running server
        stop();
        assert!(!is_running());
    }

    #[test]
    fn test_is_running_when_not_started() {
        stop();
        thread::sleep(Duration::from_millis(50));
        assert!(!is_running());
    }

    #[test]
    fn test_get_hub_when_not_running() {
        stop();
        thread::sleep(Duration::from_millis(50));
        assert!(get_hub().is_none());
    }
}
