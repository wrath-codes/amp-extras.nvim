//! Server event notifications
//!
//! Fires autocommands when server state changes (client connect/disconnect)

/// Fire autocmd when a client connects (from main thread)
#[cfg(not(test))]
pub(super) fn notify_client_connected_sync() {
    use nvim_oxi::{api, print};

    // Notify user with print!
    print!("Amp CLI:  Connected");

    // Fire User autocommand
    let _ = api::exec_autocmds(
        vec!["User"],
        &api::opts::ExecAutocmdsOpts::builder()
            .patterns(vec!["AmpClientConnected"])
            .build(),
    );
}

/// Fire autocmd when a client disconnects (from main thread)
#[cfg(not(test))]
pub(super) fn notify_client_disconnected_sync() {
    use nvim_oxi::{api, print};

    // Notify user with print! instead of api::notify (deprecated)
    print!("Amp CLI:  Disconnected");

    // Fire User autocommand (ignore errors - best effort)
    let _ = api::exec_autocmds(
        vec!["User"],
        &api::opts::ExecAutocmdsOpts::builder()
            .patterns(vec!["AmpClientDisconnected"])
            .build(),
    );
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
