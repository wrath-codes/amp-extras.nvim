//! Hub for managing multiple WebSocket connections

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use tokio::sync::mpsc::UnboundedSender;

use crate::errors::{AmpError, Result};

pub type ClientId = u64;

static NEXT_CLIENT_ID: AtomicU64 = AtomicU64::new(1);

pub type ClientMessage = String;

/// Hub for managing multiple WebSocket connections
#[derive(Clone, Debug)]
pub struct Hub {
    clients: Arc<Mutex<HashMap<ClientId, UnboundedSender<ClientMessage>>>>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn next_client_id() -> ClientId {
        NEXT_CLIENT_ID.fetch_add(1, Ordering::SeqCst)
    }

    pub fn register(&self, id: ClientId, sender: UnboundedSender<ClientMessage>) {
        let mut clients = self.clients.lock().unwrap();
        clients.insert(id, sender);
    }

    pub fn unregister(&self, id: ClientId) {
        let mut clients = self.clients.lock().unwrap();
        clients.remove(&id);
    }

    pub fn client_count(&self) -> usize {
        self.clients.lock().unwrap().len()
    }

    pub fn broadcast(&self, message: &str) {
        let clients = self.clients.lock().unwrap();
        for (_id, sender) in clients.iter() {
            let _ = sender.send(message.to_string());
        }
    }

    pub fn send_to_client(&self, id: ClientId, message: &str) -> Result<()> {
        let clients = self.clients.lock().unwrap();
        if let Some(sender) = clients.get(&id) {
            sender
                .send(message.to_string())
                .map_err(|_| AmpError::HubError("Failed to send message to client".to_string()))?;
            Ok(())
        } else {
            Err(AmpError::HubError(format!("Client {} not found", id)))
        }
    }
}
