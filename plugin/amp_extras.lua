-- amp-extras: Neovim plugin initialization
--
-- User commands are registered in Rust (see crates/core/src/lib.rs)
-- This file handles Lua-side initialization only

-- Note: All user commands (AmpSend*, AmpServer*) are now registered directly
-- in Rust via the #[nvim_oxi::plugin] macro. See crates/core/src/lib.rs for details.
--
-- This file is kept minimal - only for future Lua-specific initialization if needed.
