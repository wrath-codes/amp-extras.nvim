-- amp-extras: Neovim plugin for Amp CLI integration
local M = {}

-- Load FFI module (separated for cleaner integration with UI commands)
local ffi = require("amp_extras.ffi")

-- ============================================================================
-- State Table (mirrors amp.nvim pattern)
-- ============================================================================

--- Server state (updated by get_status)
M.state = {
  running = false,
  port = nil,
  clients = 0,
}

-- Expose FFI methods on M for backward compatibility
M.call = ffi.call
M.server_start = ffi.server_start
M.server_stop = ffi.server_stop
M.server_is_running = ffi.server_is_running
M.setup_notifications = ffi.setup_notifications
M.autocomplete = ffi.autocomplete
M.send_user_message = ffi.send_user_message
M.send_to_prompt = ffi.send_to_prompt

-- Default configuration
local defaults = {
  auto_start = true, -- Auto-start server on setup
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

    -- Server commands
    server_start = true, -- default: prefix .. "xs"
    server_stop = true, -- default: prefix .. "xx"
    server_status = true, -- default: prefix .. "xc"
    update = true, -- default: prefix .. "u"
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
  server_start = "xs",
  server_stop = "xx",
  server_status = "xc",
  update = "u",
}

-- Active configuration (merged defaults + user config)
M.config = vim.deepcopy(defaults)

-- ============================================================================
-- Server Status API
-- ============================================================================

--- Get server status and update state table
---
--- Calls the Rust server.status command and updates M.state.
--- Can return either a table or a formatted string for statusline.
---
---@param opts? table Options
---   - return_string (boolean): Return formatted string instead of table
---@return table|string status Server status (or formatted string if return_string=true)
function M.get_status(opts)
  opts = opts or {}

  -- Call Rust command
  local ok, res = pcall(ffi.call, "server.status", {})
  if ok and type(res) == "table" then
    M.state.running = res.running or false
    M.state.port = res.port
    M.state.clients = res.clients or 0
  else
    M.state.running = false
    M.state.port = nil
    M.state.clients = 0
  end

  -- Return formatted string for statusline
  if opts.return_string then
    if M.state.running then
      local port = M.state.port or 0
      local clients = M.state.clients or 0
      return string.format("Amp:%d[%d]", port, clients)
    else
      return "Amp:off"
    end
  end

  -- Return copy of state table
  return vim.deepcopy(M.state)
end

--- Simple statusline component
---
--- Returns formatted status string suitable for statusline/lualine.
--- Format: "Amp:PORT[CLIENTS]" when running, "Amp:off" when stopped.
---
---@return string Formatted status string
function M.statusline_component()
  return M.get_status({ return_string = true })
end

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

  -- Server commands
  map("server_start", "n", "<cmd>AmpServerStart<cr>", "Amp: Start Server")
  map("server_stop", "n", "<cmd>AmpServerStop<cr>", "Amp: Stop Server")
  map("server_status", "n", "<cmd>AmpServerStatus<cr>", "Amp: Server Status")

  -- Update
  map("update", "n", "<cmd>AmpUpdate<cr>", "Amp: Update CLI")
end

--- Setup amp-extras plugin
---
--- Merges user configuration with defaults and initializes the plugin.
--- If auto_start is true, starts the WebSocket server automatically.
---
---@param opts table|nil User configuration options
---   - auto_start (boolean): Auto-start server on setup (default: true)
---   - lazy (boolean): Lazy load the plugin (default: false)
---   - prefix (string): Prefix for default keymaps (default: "<leader>a")
---   - keymaps (table|false): Keymap configuration (set to false to disable all)
---@return table Configuration
function M.setup(opts)
  -- Merge user config with defaults
  opts = opts or {}
  sM.config = vim.tbl_deep_extend("force", defaults, opts)

  -- Call Rust FFI setup to register VimEnter autocommand if auto_start is enabled
  local setup_result = ffi.setup({ auto_start = M.config.auto_start })
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
