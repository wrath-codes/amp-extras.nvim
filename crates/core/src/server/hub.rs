//! Connection registry and broadcast support

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::Sender;

use crate::errors::{AmpError, Result};

/// Client ID type
pub type ClientId = u64;

/// Global client ID counter
static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);

/// Message to send to a client
pub type ClientMessage = String;

/// Hub for managing multiple WebSocket connections
#[derive(Clone)]
pub struct Hub {
    clients: Arc<Mutex<HashMap<ClientId, Sender<ClientMessage>>>>,
}

impl Hub {
    /// Create a new hub
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Generate a unique client ID
    pub fn next_client_id() -> ClientId {
        NEXT_CLIENT_ID.fetch_add(1, Ordering::SeqCst)
    }
    
    /// Register a new client with its message sender
    pub fn register(&self, id: ClientId, sender: Sender<ClientMessage>) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(id, sender);
    }
    
    /// Unregister a client
    pub fn unregister(&self, id: ClientId) {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(&id);
    }
    
    /// Get count of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }
    
    /// Broadcast a message to all connected clients
    ///
    /// Sends the message to all registered clients. If sending fails for any client,
    /// that client is silently skipped (they may have disconnected).
    pub fn broadcast(&self, message: &str) {
        let clients = self.clients.lock().unwrap();
        
        for (_id, sender) in clients.iter() {
            let _ = sender.try_send(message.to_string());
        }
    }
    
    /// Send a message to a specific client
    pub fn send_to_client(&self, id: ClientId, message: &str) -> Result<()> {
        let clients = self.clients.lock().unwrap();
        
        if let Some(sender) = clients.get(&id) {
            sender.try_send(message.to_string())
                .map_err(|_| AmpError::HubError("Failed to send message to client".to_string()))?;
            Ok(())
        } else {
            Err(AmpError::HubError(format!("Client {} not found", id)))
        }
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn test_hub_new() {
        let hub = Hub::new();
        assert_eq!(hub.client_count(), 0);
    }

    #[test]
    fn test_register_and_unregister() {
        let hub = Hub::new();
        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();

        // Create real channels for testing
        let (tx1, _rx1) = unbounded();
        let (tx2, _rx2) = unbounded();

        hub.register(id1, tx1);
        assert_eq!(hub.client_count(), 1);

        hub.register(id2, tx2);
        assert_eq!(hub.client_count(), 2);

        hub.unregister(id1);
        assert_eq!(hub.client_count(), 1);

        hub.unregister(id2);
        assert_eq!(hub.client_count(), 0);
    }

    #[test]
    fn test_unique_client_ids() {
        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();
        let id3 = Hub::next_client_id();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_unregister_nonexistent() {
        let hub = Hub::new();
        // Should not panic when unregistering non-existent client
        hub.unregister(999);
        assert_eq!(hub.client_count(), 0);
    }

    #[test]
    fn test_broadcast() {
        let hub = Hub::new();
        
        // Create 3 clients with real channels
        let (tx1, rx1) = unbounded();
        let (tx2, rx2) = unbounded();
        let (tx3, rx3) = unbounded();
        
        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();
        let id3 = Hub::next_client_id();
        
        hub.register(id1, tx1);
        hub.register(id2, tx2);
        hub.register(id3, tx3);
        
        // Broadcast a message
        hub.broadcast("test message");
        
        // All clients should receive it
        assert_eq!(rx1.try_recv().unwrap(), "test message");
        assert_eq!(rx2.try_recv().unwrap(), "test message");
        assert_eq!(rx3.try_recv().unwrap(), "test message");
    }

    #[test]
    fn test_send_to_client() {
        let hub = Hub::new();
        
        let (tx1, rx1) = unbounded();
        let (tx2, rx2) = unbounded();
        
        let id1 = Hub::next_client_id();
        let id2 = Hub::next_client_id();
        
        hub.register(id1, tx1);
        hub.register(id2, tx2);
        
        // Send to specific client
        hub.send_to_client(id1, "message for client 1").unwrap();
        
        // Only client 1 should receive it
        assert_eq!(rx1.try_recv().unwrap(), "message for client 1");
        assert!(rx2.try_recv().is_err()); // Client 2 should have nothing
    }

    #[test]
    fn test_send_to_nonexistent_client() {
        let hub = Hub::new();
        
        let result = hub.send_to_client(999, "test");
        assert!(result.is_err());
        
        match result {
            Err(AmpError::HubError(msg)) => {
                assert!(msg.contains("Client 999 not found"));
            }
            _ => panic!("Expected HubError"),
        }
    }
}
