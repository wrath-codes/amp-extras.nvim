-- FFI interface to Rust core library
local M = {}

-- Determine the directory where this module resides
local source = debug.getinfo(1, "S").source
local module_dir = source:match("@(.*/)") -- Extract directory from "@/path/to/ffi.lua"

-- Add module directory to package.cpath so it can find amp_extras_core.so
if module_dir then
  package.cpath = module_dir .. "?.so;" .. package.cpath
end

-- Load the compiled Rust FFI module
local ffi = require("amp_extras_core")

-- ============================================================================
-- Command Interface
-- ============================================================================

--- Call a command through the FFI
---@param command string Command name (e.g., "ping", "send_selection")
---@param args table Command arguments
---@return table Result or error object
function M.call(command, args)
  args = args or {}
  local result = ffi.call(command, args)

  -- Check if result is an error
  if result.error then
    return { nil, result.message }
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

--- Setup notification autocommands
---
--- Registers autocmds in Rust for:
--- - selectionDidChange (CursorMoved/CursorMovedI with 10ms debouncing)
--- - visibleFilesDidChange (BufEnter/WinEnter with 10ms debouncing)
---
---@return table|nil result Success status or nil on error
---@return string|nil error Error message if failed
function M.setup_notifications()
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
-- User Message Interface
-- ============================================================================

--- Send user message to agent
---
--- Sends a message directly to the Amp agent (immediately submits).
--- Requires WebSocket server to be running.
---
---@param message string Message text to send to agent
---@return table|nil result Success status or nil on error
---@return string|nil error Error message if failed
function M.send_user_message(message)
  local result = ffi.send_user_message(message)

  if result.error then
    return nil, result.message
  end

  return result
end

--- Append text to IDE prompt field
---
--- Appends text to Amp IDE's prompt field without sending.
--- Allows user to edit before submitting.
--- Requires WebSocket server to be running.
---
---@param message string Text to append to prompt field
---@return table|nil result Success status or nil on error
---@return string|nil error Error message if failed
function M.send_to_prompt(message)
  local result = ffi.send_to_prompt(message)

  if result.error then
    return nil, result.message
  end

  return result
end

-- ============================================================================
-- Plugin Setup Interface
-- ============================================================================

--- Setup the plugin with configuration
---
--- Registers VimEnter autocommand if auto_start is enabled.
---
---@param config table Configuration options
---   - auto_start (boolean): Auto-start server on VimEnter
---@return table result Success status or error object
function M.setup(config)
  return ffi.setup(config or {})
end

return M
