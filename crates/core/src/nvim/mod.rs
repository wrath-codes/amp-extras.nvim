//! Neovim API helpers built on nvim-oxi
//!
//! This module provides reusable utilities for common Neovim operations,
//! organized by domain:
//!
//! - `cursor` - Cursor position and movement
//! - `buffer` - Buffer content and metadata
//! - `selection` - Visual selections and marks
//! - `path` - Path and URI conversions
//! - `window` - Window operations

pub mod buffer;
pub mod cursor;
pub mod path;
pub mod selection;
pub mod window;
