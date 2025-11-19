-- amp-extras: Neovim plugin for Amp CLI integration
local M = {}

-- Load FFI module (separated for cleaner integration with UI commands)
local ffi = require("amp_extras.ffi")

-- Expose FFI methods on M for backward compatibility
M.call = ffi.call
M.autocomplete = ffi.autocomplete

-- Default configuration
local defaults = {
  lazy = false, -- Lazy load the plugin
  prefix = "<leader>a", -- Prefix for all default keymaps

  -- Keymap configuration
  -- Set to false to disable all keymaps
  -- Set specific keys to false to disable individual keymaps
  -- Set specific keys to string to override default mapping
  keymaps = {
    -- Send commands
    send_selection = true, -- default: prefix .. "sh"
    send_selection_ref = true, -- default: prefix .. "sl"
    send_buffer = true, -- default: prefix .. "sb"
    send_file_ref = true, -- default: prefix .. "sf"
    send_line_ref = true, -- default: prefix .. "sr"
    send_message = true, -- default: prefix .. "sm"
  },
}

-- Default suffixes for keymaps
local default_suffixes = {
  send_selection = "sh",
  send_selection_ref = "sl",
  send_buffer = "sb",
  send_file_ref = "sf",
  send_line_ref = "sr",
  send_message = "sm",
}

-- Active configuration (merged defaults + user config)
M.config = vim.deepcopy(defaults)

-- ============================================================================
-- Setup & Configuration
-- ============================================================================

--- Resolve keymap: user_val -> final_lhs or nil
---@param action string Action name (e.g. "send_selection")
---@param user_val boolean|string|nil User config value for this action
---@param prefix string Global prefix
---@return string|nil lhs The resolved keymap string, or nil if disabled
local function resolve_keymap(action, user_val, prefix)
  if user_val == false then
    return nil
  end

  if type(user_val) == "string" then
    return user_val
  end

  if user_val == true or user_val == nil then
    local suffix = default_suffixes[action]
    if suffix then
      return prefix .. suffix
    end
  end

  return nil
end

--- Setup keymaps
---@param config table Full configuration table
local function setup_keymaps(config)
  local keymaps = config.keymaps
  local prefix = config.prefix

  if not keymaps or keymaps == false then
    return
  end

  -- Helper to map if resolved
  local function map(action, mode, rhs, desc)
    local lhs = resolve_keymap(action, keymaps[action], prefix)
    if lhs then
      vim.keymap.set(mode, lhs, rhs, {
        noremap = true,
        silent = true,
        desc = desc,
      })
    end
  end

  -- Send selection (visual mode)
  map("send_selection", "v", ":'<,'>AmpSendSelection<cr>", "Amp: Send Selection (Content)")

  -- Send selection reference (visual mode)
  map("send_selection_ref", "v", ":'<,'>AmpSendSelectionRef<cr>", "Amp: Send Selection (Ref)")

  -- Send buffer (normal mode)
  map("send_buffer", "n", "<cmd>AmpSendBuffer<cr>", "Amp: Send Buffer (Content)")

  -- Send file reference (normal mode)
  map("send_file_ref", "n", "<cmd>AmpSendFileRef<cr>", "Amp: Send File (Ref)")

  -- Send line reference (normal mode)
  map("send_line_ref", "n", "<cmd>AmpSendLineRef<cr>", "Amp: Send Line (Ref)")

  -- Send message UI
  map("send_message", "n", "<cmd>AmpSendMessage<cr>", "Amp: Send Message UI")
end

--- Setup amp-extras plugin
---
--- Merges user configuration with defaults and initializes the plugin.
---
---@param opts table|nil User configuration options
---   - lazy (boolean): Lazy load the plugin (default: false)
---   - prefix (string): Prefix for default keymaps (default: "<leader>a")
---   - keymaps (table|false): Keymap configuration (set to false to disable all)
---@return table Configuration
function M.setup(opts)
  -- Merge user config with defaults
  opts = opts or {}
  M.config = vim.tbl_deep_extend("force", defaults, opts)

  -- Call Rust FFI setup
  local setup_result = ffi.setup({})
  if setup_result and setup_result.error then
    vim.notify(
      "amp-extras: FFI setup failed: " .. (setup_result.message or "unknown error"),
      vim.log.levels.ERROR
    )
  end

  -- Setup keymaps
  setup_keymaps(M.config)

  -- Register UI commands
  M.register_ui_commands()

  return M.config
end

--- Register UI commands (can also be called manually)
function M.register_ui_commands()
  local commands = require("amp_extras.commands")
  commands.register_commands()
end

return M
