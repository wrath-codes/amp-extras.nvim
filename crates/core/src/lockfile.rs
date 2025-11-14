//! Lockfile management for WebSocket server discovery
//!
//! Writes lockfiles to ~/.local/share/amp/ide/<port>.json with:
//! - port: Server port number
//! - auth: Authentication token
//! - pid: Process ID
//! - version: Plugin version
//! - startedAt: ISO 8601 timestamp

use std::{fs, path::PathBuf};

use nvim_oxi::api;
use serde::{Deserialize, Serialize};

use crate::errors::{AmpError, Result};

/// Lockfile JSON structure
///
/// Matches amp.nvim lockfile format for Amp CLI compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lockfile {
    pub port:              u16,
    pub auth_token:        String,
    pub pid:               u32,
    pub workspace_folders: Vec<String>,
    pub ide_name:          String,
}

/// Generate a random authentication token
pub fn generate_token(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

    let mut rng = rand::rng();
    (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Get lockfile directory path (~/.local/share/amp/ide/)
///
/// Always uses ~/.local/share/amp/ide for Amp CLI compatibility,
/// even on macOS (where dirs crate would use ~/Library/Application Support)
pub fn lockfile_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| AmpError::ConfigError("Could not determine home directory".into()))?;

    // Force ~/.local/share/amp/ide for cross-platform Amp CLI compatibility
    let amp_ide_dir = home.join(".local").join("share").join("amp").join("ide");
    Ok(amp_ide_dir)
}

/// Write lockfile with server information
///
/// Creates ~/.local/share/amp/ide/<port>.json with server metadata
pub fn write_lockfile(port: u16, token: &str) -> Result<PathBuf> {
    let dir = lockfile_dir()?;

    // Create directory if it doesn't exist
    fs::create_dir_all(&dir)?;

    let lockfile_path = dir.join(format!("{}.json", port));

    // Get current working directory for workspaceFolders
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| String::from("/"));

    // Get Neovim version for ideName
    let nvim_version = get_nvim_version();

    let lockfile = Lockfile {
        port,
        auth_token: token.to_string(),
        pid: std::process::id(),
        workspace_folders: vec![cwd],
        ide_name: nvim_version,
    };

    let json = serde_json::to_string_pretty(&lockfile)?;
    fs::write(&lockfile_path, json)?;

    Ok(lockfile_path)
}

/// Remove lockfile
pub fn remove_lockfile(path: &PathBuf) -> Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Get Neovim version string
///
/// Uses Vimscript to call Lua's vim.version() and formats as "nvim X.Y.Z"
fn get_nvim_version() -> String {
    // Use eval with Vimscript's v:lua to call vim.version()
    // We get major, minor, patch separately since we can't easily pass a dictionary
    let major: i64 = api::eval("v:lua.vim.version().major").unwrap_or(0);
    let minor: i64 = api::eval("v:lua.vim.version().minor").unwrap_or(0);
    let patch: i64 = api::eval("v:lua.vim.version().patch").unwrap_or(0);

    if major == 0 && minor == 0 && patch == 0 {
        // All zeros means the call failed, use fallback
        String::from("nvim")
    } else {
        format!("nvim {}.{}.{}", major, minor, patch)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn test_generate_token_length() {
        let token = generate_token(32);
        assert_eq!(token.len(), 32);
    }

    #[test]
    fn test_generate_token_charset() {
        let token = generate_token(100);
        // Should only contain alphanumeric characters
        assert!(token.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_generate_token_randomness() {
        // Generate two tokens and verify they're different
        let token1 = generate_token(32);
        let token2 = generate_token(32);
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_lockfile_dir() {
        let dir = lockfile_dir().unwrap();
        // Should end with amp/ide
        assert!(dir.ends_with("amp/ide"));
    }

    #[test]
    fn test_write_and_remove_lockfile() {
        // Use a temporary directory to avoid polluting the real lockfile dir
        let temp_dir = TempDir::new().unwrap();
        let lockfile_path = temp_dir.path().join("12345.json");

        // Write lockfile
        let token = "test_token_123456";
        let lockfile = Lockfile {
            port:              12345,
            auth_token:        token.to_string(),
            pid:               std::process::id(),
            workspace_folders: vec!["/tmp".to_string()],
            ide_name:          "nvim 0.10".to_string(),
        };

        let json = serde_json::to_string_pretty(&lockfile).unwrap();
        fs::write(&lockfile_path, json).unwrap();

        // Verify it exists
        assert!(lockfile_path.exists());

        // Read and verify content
        let content = fs::read_to_string(&lockfile_path).unwrap();
        let parsed: Lockfile = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.port, 12345);
        assert_eq!(parsed.auth_token, token);
        assert_eq!(parsed.workspace_folders[0], "/tmp");

        // Remove lockfile
        remove_lockfile(&lockfile_path).unwrap();

        // Verify it's gone
        assert!(!lockfile_path.exists());
    }

    #[test]
    fn test_remove_nonexistent_lockfile() {
        let temp_dir = TempDir::new().unwrap();
        let fake_path = temp_dir.path().join("nonexistent.json");

        // Should not error when removing non-existent file
        let result = remove_lockfile(&fake_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lockfile_json_format() {
        let lockfile = Lockfile {
            port:              54321,
            auth_token:        "abc123".to_string(),
            pid:               99999,
            workspace_folders: vec!["/home/user".to_string()],
            ide_name:          "nvim 0.10".to_string(),
        };

        let json = serde_json::to_string(&lockfile).unwrap();

        // Verify camelCase fields (matches amp.nvim format)
        assert!(json.contains("\"port\":"));
        assert!(json.contains("\"authToken\":"));
        assert!(json.contains("\"pid\":"));
        assert!(json.contains("\"workspaceFolders\":"));
        assert!(json.contains("\"ideName\":"));
    }

    #[test]
    #[ignore = "Requires Neovim context for nvim_oxi::api::get_version()"]
    fn test_write_lockfile_creates_directory() {
        // This test actually writes to the real lockfile directory
        // but cleans up after itself
        let token = generate_token(32);
        let port = 65000; // Use high port to avoid conflicts

        let lockfile_path = write_lockfile(port, &token).unwrap();

        // Verify it was created
        assert!(lockfile_path.exists());

        // Verify content
        let content = fs::read_to_string(&lockfile_path).unwrap();
        let parsed: Lockfile = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.port, port);
        assert_eq!(parsed.auth_token, token);
        assert!(!parsed.workspace_folders.is_empty());
        assert!(parsed.ide_name.contains("nvim"));

        // Clean up
        remove_lockfile(&lockfile_path).unwrap();
    }
}
