//! WebSocket server accept loop and connection spawning

use std::{
    io::ErrorKind,
    net::TcpListener,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::{connection, hub::Hub};

/// Start the WebSocket accept loop
///
/// Binds to 127.0.0.1:0 (random port) and accepts incoming connections
pub fn run_accept_loop(listener: TcpListener, token: String, hub: Hub, shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _addr)) => {
                // Spawn connection handler thread
                let token_clone = token.clone();
                let hub_clone = hub.clone();
                std::thread::spawn(move || {
                    connection::handle_connection(stream, token_clone, hub_clone);
                });
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // No incoming connections, sleep briefly
                std::thread::sleep(Duration::from_millis(25));
            },
            Err(_e) => {
                // Accept error - sleep and continue (connection handler will log errors)
                std::thread::sleep(Duration::from_millis(100));
            },
        }
    }
}
