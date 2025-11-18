//! WebSocket server accept loop (async with Tokio)

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tokio::net::TcpListener;

use super::{connection_async, hub::Hub};

/// Run the async accept loop
///
/// Listens for incoming connections and spawns async tasks to handle them.
pub async fn run_accept_loop(
    listener: TcpListener,
    token: String,
    hub: Hub,
    shutdown: Arc<AtomicBool>,
) {
    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        match listener.accept().await {
            Ok((stream, _addr)) => {
                let token_clone = token.clone();
                let hub_clone = hub.clone();

                tokio::spawn(async move {
                    connection_async::handle_connection(stream, token_clone, hub_clone).await;
                });
            }
            Err(_) => {
                // Accept error, continue
            }
        }
    }
}
