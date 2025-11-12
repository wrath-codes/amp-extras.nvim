-- amp-extras: Neovim plugin for Amp CLI integration
-- Lua interface to Rust FFI

local M = {}

-- Load the Rust FFI library
-- Build output: target/*/libamp_extras_core.{dylib,so,dll}
-- Copied by build.rs to: lua/amp_extras/amp_extras_core.so (debug builds)
-- Copied by justfile to: lua/amp_extras/amp_extras.so (release builds via `just build`)
local ffi = require("amp_extras_core")

-- ============================================================================
-- Command Interface
-- ============================================================================

--- Call a command through the FFI
---@param command string Command name (e.g., "ping", "threads.list")
---@param args table Command arguments
---@return table Result or error object
function M.call(command, args)
  args = args or {}
  local result = ffi.call(command, args)
  
  -- Check if result is an error
  if result.error then
    return nil, result.message
  end
  
  return result
end

-- ============================================================================
-- WebSocket Server Interface
-- ============================================================================

--- Start the WebSocket server
---@return table|nil result Server info (port, token, lockfile) or nil on error
---@return string|nil error Error message if failed
function M.server_start()
  local result = ffi.server_start()
  
  if result.error then
    return nil, result.message
  end
  
  return result
end

--- Stop the WebSocket server
---@return boolean success
function M.server_stop()
  local result = ffi.server_stop()
  return result.success == true
end

--- Check if WebSocket server is running
---@return boolean running
function M.server_is_running()
  local result = ffi.server_is_running()
  return result.running == true
end

--- Setup notification autocommands (Lua-based with debouncing)
---@return table|nil result Success status or nil on error
---@return string|nil error Error message if failed
function M.setup_notifications()
  -- Call Rust FFI to setup autocmds with debouncing
  -- Uses nvim-oxi TimerHandle for 10ms debouncing on libuv event loop
  return ffi.setup_notifications()
end

-- ============================================================================
-- Autocomplete Interface
-- ============================================================================

--- Get autocomplete suggestions
---@param kind string Type of completion ("thread", "prompt", "file")
---@param prefix string User-typed prefix
---@return string[] Completion items
function M.autocomplete(kind, prefix)
  return ffi.autocomplete(kind, prefix)
end

-- ============================================================================
-- Notification Interface (for internal use by selection/visible_files modules)
-- ============================================================================

--- Send selection changed notification
---@param uri string File URI
---@param start_line number Start line (0-indexed)
---@param start_char number Start character (0-indexed)
---@param end_line number End line (0-indexed)
---@param end_char number End character (0-indexed)
---@param content string Selected text content
---@return table result
function M.send_selection_changed(uri, start_line, start_char, end_line, end_char, content)
  return ffi.send_selection_changed(uri, start_line, start_char, end_line, end_char, content)
end

--- Send visible files changed notification
---@param uris string[] List of file URIs
---@return table result
function M.send_visible_files_changed(uris)
  return ffi.send_visible_files_changed(uris)
end

return M
