//! Neovim autocommand setup for WebSocket notifications
//!
//! This module coordinates autocommands that trigger WebSocket notifications
//! when Neovim events occur. Each autocommand type is in its own file for
//! easy extension and maintenance.

use nvim_oxi::api::{self, opts::CreateAugroupOpts};

use crate::{errors::Result, server::Hub};

// One module per autocommand type
mod diagnostics_changed;
mod selection_changed;
mod visible_files_changed;

// Re-export handlers for internal use (initial state broadcast)
// Used by server/connection.rs for sending initial state to newly connected
// clients
#[allow(unused_imports)]
pub(crate) use diagnostics_changed::handle_event as handle_diagnostics_changed;
#[allow(unused_imports)]
pub(crate) use selection_changed::handle_event as handle_cursor_moved;
#[allow(unused_imports)]
pub(crate) use visible_files_changed::handle_event as handle_visible_files_changed;

/// Autocommand group name for amp-extras notifications
const AUGROUP_NAME: &str = "AmpExtrasNotifications";

/// Setup all notification autocommands
///
/// Creates an autocommand group and registers all notification autocmds.
/// This is the main entry point for the autocmds module.
///
/// # Arguments
/// * `hub` - WebSocket Hub for broadcasting notifications
///
/// # Returns
/// * `Ok(())` if setup succeeded
/// * `Err(AmpError)` if autocommand creation failed
pub fn setup_notifications(hub: Hub) -> Result<()> {
    // Create autocommand group (clear existing if present)
    let group_opts = CreateAugroupOpts::builder().clear(true).build();

    let group_id = api::create_augroup(AUGROUP_NAME, &group_opts)
        .map_err(|e| crate::errors::AmpError::Other(format!("Failed to create augroup: {}", e)))?;

    // Register each autocommand type
    diagnostics_changed::register(group_id, std::sync::Arc::new(hub.clone()))?;
    selection_changed::register(group_id, std::sync::Arc::new(hub.clone()))?;
    visible_files_changed::register(group_id, std::sync::Arc::new(hub))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_augroup_name() {
        assert_eq!(AUGROUP_NAME, "AmpExtrasNotifications");
    }

    #[test]
    fn test_debounce_constants() {
        assert_eq!(diagnostics_changed::DEBOUNCE_MS, 10);
        assert_eq!(selection_changed::DEBOUNCE_MS, 10);
        assert_eq!(visible_files_changed::DEBOUNCE_MS, 10);
    }

    // Note: Actual autocommand tests require Neovim context
    // and should be run as integration tests
}
