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
-- Plugin Setup Interface
-- ============================================================================

--- Setup the plugin with configuration
---
---@param config table Configuration options
---@return table result Success status or error object
function M.setup(config)
  return ffi.setup(config or {})
end

return M