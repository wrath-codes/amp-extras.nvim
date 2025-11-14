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
  keymaps = {
    -- Send commands (set to false to disable)
    send_selection = "<leader>ash", -- Send selection content (visual)
    send_selection_ref = "<leader>asl", -- Send selection reference (visual)
    send_buffer = "<leader>asb", -- Send buffer content
    send_file_ref = "<leader>asf", -- Send file reference
    send_line_ref = "<leader>asr", -- Send line reference
    send_message = "<leader>asm", -- Send message UI
    -- Server commands (set to false to disable)
    server_start = "<leader>axs", -- Start WebSocket server
    server_stop = "<leader>axx", -- Stop WebSocket server
    server_status = "<leader>axc", -- Server status
  },
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

--- Setup keymaps
---@param keymaps table Keymap configuration
local function setup_keymaps(keymaps)
  if not keymaps or keymaps == false then
    return
  end

  -- Send selection (visual mode)
  if keymaps.send_selection then
    vim.keymap.set("v", keymaps.send_selection, ":'<,'>AmpSendSelection<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Send Selection (Content)",
    })
  end

  -- Send selection reference (visual mode)
  if keymaps.send_selection_ref then
    vim.keymap.set("v", keymaps.send_selection_ref, ":'<,'>AmpSendSelectionRef<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Send Selection (Ref)",
    })
  end

  -- Send buffer (normal mode)
  if keymaps.send_buffer then
    vim.keymap.set("n", keymaps.send_buffer, "<cmd>AmpSendBuffer<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Send Buffer (Content)",
    })
  end

  -- Send file reference (normal mode)
  if keymaps.send_file_ref then
    vim.keymap.set("n", keymaps.send_file_ref, "<cmd>AmpSendFileRef<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Send File (Ref)",
    })
  end

  -- Send line reference (normal mode)
  if keymaps.send_line_ref then
    vim.keymap.set("n", keymaps.send_line_ref, "<cmd>AmpSendLineRef<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Send Line (Ref)",
    })
  end

  -- Server start
  if keymaps.server_start then
    vim.keymap.set("n", keymaps.server_start, "<cmd>AmpServerStart<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Start Server",
    })
  end

  -- Server stop
  if keymaps.server_stop then
    vim.keymap.set("n", keymaps.server_stop, "<cmd>AmpServerStop<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Stop Server",
    })
  end

  -- Server status
  if keymaps.server_status then
    vim.keymap.set("n", keymaps.server_status, "<cmd>AmpServerStatus<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Server Status",
    })
  end

  -- Send message UI
  if keymaps.send_message then
    vim.keymap.set("n", keymaps.send_message, "<cmd>AmpSendMessage<cr>", {
      noremap = true,
      silent = true,
      desc = "Amp: Send Message UI",
    })
  end
end

--- Setup amp-extras plugin
---
--- Merges user configuration with defaults and initializes the plugin.
--- If auto_start is true, starts the WebSocket server automatically.
---
---@param opts table|nil User configuration options
---   - auto_start (boolean): Auto-start server on setup (default: true)
---   - lazy (boolean): Lazy load the plugin (default: false)
---   - keymaps (table|false): Keymap configuration (set to false to disable all)
---@return table Configuration
function M.setup(opts)
  -- Merge user config with defaults
  opts = opts or {}
  M.config = vim.tbl_deep_extend("force", defaults, opts)

  -- Auto-start server if enabled
  if M.config.auto_start and not M.server_is_running() then
    local result, err = M.server_start()
    if not result then
      vim.notify(
        "amp-extras: Failed to start server: " .. (err or "unknown error"),
        vim.log.levels.ERROR
      )
    else
      vim.notify("amp-extras: Server started on port " .. result.port, vim.log.levels.INFO)

      -- Setup notifications (cursor/selection tracking)
      M.setup_notifications()
    end
  end

  -- Setup keymaps
  setup_keymaps(M.config.keymaps)

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
