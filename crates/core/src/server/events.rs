//! Server event notifications
//!
//! Fires autocommands when server state changes (client connect/disconnect)

/// Fire autocmd when a client connects
#[cfg(not(test))]
pub(super) fn notify_client_connected() {
    // Check if Neovim is available before scheduling work
    if !crate::ide_ops::nvim_available() {
        return;
    }

    let _ = crate::ide_ops::schedule_on_main_thread(|| {
        use nvim_oxi::api;

        // Fire User autocommand
        let _ = api::exec_autocmds(
            vec!["User"],
            &api::opts::ExecAutocmdsOpts::builder()
                .patterns(vec!["AmpClientConnected"])
                .build(),
        );
    });
}

/// Fire autocmd when a client disconnects
#[cfg(not(test))]
pub(super) fn notify_client_disconnected() {
    // Check if Neovim is still available before trying to schedule work
    // This prevents crashes when Amp CLI disconnects during Neovim shutdown
    if !crate::ide_ops::nvim_available() {
        return;
    }

    // Schedule on main thread with best-effort error handling
    // If Neovim is shutting down, this will silently fail
    let _ = crate::ide_ops::schedule_on_main_thread(|| {
        use nvim_oxi::api;

        // Notify user (ignore errors - best effort during shutdown)
        let _ = api::notify(
            "Amp CLI: Disconnected",
            api::types::LogLevel::Info,
            &Default::default(),
        );

        // Fire User autocommand (ignore errors - best effort)
        let _ = api::exec_autocmds(
            vec!["User"],
            &api::opts::ExecAutocmdsOpts::builder()
                .patterns(vec!["AmpClientDisconnected"])
                .build(),
        );
    });
}

/// Fire autocmd when server starts
#[cfg(not(test))]
pub fn notify_server_started() {
    // Check if Neovim is available before scheduling work
    if !crate::ide_ops::nvim_available() {
        return;
    }

    let _ = crate::ide_ops::schedule_on_main_thread(|| {
        use nvim_oxi::api;

        let _ = api::exec_autocmds(
            vec!["User"],
            &api::opts::ExecAutocmdsOpts::builder()
                .patterns(vec!["AmpServerStarted"])
                .build(),
        );
    });
}

/// Fire autocmd when server stops
#[cfg(not(test))]
pub fn notify_server_stopped() {
    // Check if Neovim is available before scheduling work
    if !crate::ide_ops::nvim_available() {
        return;
    }

    let _ = crate::ide_ops::schedule_on_main_thread(|| {
        use nvim_oxi::api;

        let _ = api::exec_autocmds(
            vec!["User"],
            &api::opts::ExecAutocmdsOpts::builder()
                .patterns(vec!["AmpServerStopped"])
                .build(),
        );
    });
}
