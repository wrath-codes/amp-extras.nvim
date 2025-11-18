//! Account update command
//!
//! Runs `amp update` to update the CLI tool.

use std::pin::Pin;
use std::future::Future;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, BufReader};
use serde_json::Value;

use crate::errors::Result;
use crate::server::event_bridge::{send_event, ServerEvent, LogLevel};

/// Update Amp CLI tool
///
/// Spawns `amp update` process and streams output to client.
pub fn account_update(_args: Value) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(async move {
        send_event(ServerEvent::LogMessage(
            "Starting Amp CLI update...".to_string(),
            LogLevel::Info
        ));

        let mut child = tokio::process::Command::new("amp")
            .arg("update")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Essential for cleanup on cancellation
            .spawn()
            .map_err(|e| crate::errors::AmpError::AmpCliError(format!("Failed to spawn amp update: {}", e)))?;

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Stream stdout
        let stdout_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                send_event(ServerEvent::LogMessage(line, LogLevel::Info));
            }
        });

        // Stream stderr
        let stderr_handle = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                send_event(ServerEvent::LogMessage(line, LogLevel::Info));
            }
        });

        let status = child.wait().await
            .map_err(|e| crate::errors::AmpError::AmpCliError(format!("Failed to wait for amp update: {}", e)))?;

        // Wait for streams to finish
        let _ = stdout_handle.await;
        let _ = stderr_handle.await;

        if status.success() {
             send_event(ServerEvent::LogMessage(
                "Amp CLI updated successfully".to_string(),
                LogLevel::Info
            ));
        } else {
             send_event(ServerEvent::LogMessage(
                format!("Amp CLI update failed with exit code: {:?}", status.code()),
                LogLevel::Error
            ));
        }

        Ok(())
    })
}
