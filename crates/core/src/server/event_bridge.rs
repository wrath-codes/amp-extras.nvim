//! Event bridge between Tokio threads and Neovim main thread
//!
//! Tokio worker threads cannot call nvim_oxi::schedule() directly (no Lua state).
//! This module provides AsyncHandle + channel bridge to safely queue events.

use std::sync::OnceLock;

use nvim_oxi::libuv::AsyncHandle;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::hub::Hub;

/// Events that can be sent from Tokio threads to main thread
#[derive(Debug, Clone)]
pub enum ServerEvent {
    ClientConnected,
    ClientDisconnected,
    SendInitialState(Hub),
    LogMessage(String, LogLevel),
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

static EVENT_BRIDGE: OnceLock<(UnboundedSender<ServerEvent>, AsyncHandle)> = OnceLock::new();

/// Initialize the event bridge (call once at server start from main thread)
pub fn init() -> crate::errors::Result<()> {
    let (tx, mut rx): (UnboundedSender<ServerEvent>, UnboundedReceiver<ServerEvent>) =
        unbounded_channel();

    // Create AsyncHandle with callback that processes queued events
    let handle = AsyncHandle::new(move || {
        // Process all pending events (non-blocking)
        while let Ok(event) = rx.try_recv() {
            process_event(event);
        }
        Ok::<_, std::convert::Infallible>(())
    })
    .map_err(|e| crate::errors::AmpError::Other(format!("Failed to create AsyncHandle: {}", e)))?;

    EVENT_BRIDGE.set((tx, handle)).map_err(|_| {
        crate::errors::AmpError::Other("Event bridge already initialized".into())
    })?;

    Ok(())
}

/// Send event from Tokio thread to main thread
pub fn send_event(event: ServerEvent) {
    if let Some((tx, handle)) = EVENT_BRIDGE.get() {
        // Queue the event
        let _ = tx.send(event);
        // Wake up AsyncHandle to process it
        let _ = handle.send();
    }
}

/// Process a single event on main thread (called by AsyncHandle)
fn process_event(event: ServerEvent) {
    match event {
        ServerEvent::ClientConnected => {
            #[cfg(not(test))]
            super::events::notify_client_connected_sync();
        }
        ServerEvent::ClientDisconnected => {
            #[cfg(not(test))]
            super::events::notify_client_disconnected_sync();
        }
        ServerEvent::SendInitialState(hub) => {
            #[cfg(not(test))]
            send_initial_state_sync(hub);
        }
        ServerEvent::LogMessage(msg, level) => {
            #[cfg(not(test))]
            log_message_sync(msg, level);
        }
    }
}

/// Log message to Neovim (synchronous)
#[cfg(not(test))]
fn log_message_sync(msg: String, level: LogLevel) {
    use nvim_oxi::print;
    // You might want to map this to vim.notify with levels
    // For now, simple print with prefix
    let prefix = match level {
        LogLevel::Info => "Amp Info:",
        LogLevel::Warn => "Amp Warn:",
        LogLevel::Error => "Amp Error:",
    };
    print!("{} {}", prefix, msg);
}

/// Send initial state notifications (synchronous version for main thread)
#[cfg(not(test))]
fn send_initial_state_sync(hub: Hub) {
    if !super::is_running() || !crate::ide_ops::nvim_available() {
        return;
    }

    // Send plugin metadata
    let version = env!("CARGO_PKG_VERSION");
    let plugin_dir = env!("CARGO_MANIFEST_DIR");
    let _ = crate::notifications::send_plugin_metadata(&hub, version, plugin_dir);

    // Send initial state notifications
    if hub.client_count() > 0 && super::is_running() {
        let _ = crate::autocmds::handle_visible_files_changed(&hub);
        let _ = crate::autocmds::handle_cursor_moved(&hub);
        let _ = crate::autocmds::handle_diagnostics_changed(&hub);
    }
}
