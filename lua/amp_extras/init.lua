-- amp-extras: Neovim plugin for Amp CLI integration
local M = {}

-- Load FFI module (separated for cleaner integration with UI commands)
local ffi = require("amp_extras.ffi")

-- Expose FFI methods on M for backward compatibility
M.call = ffi.call
M.autocomplete = ffi.autocomplete

-- ============================================================================
-- Default Configuration
-- ============================================================================

local defaults = {
  lazy = false, -- Lazy load the plugin
  prefix = "a", -- Default prefix (mappings will be <leader> + prefix)

  -- Feature flags: Toggle groups of functionality
  features = {
    send = true, -- Send commands (buffer, selection, line, file)
    message = true, -- Send message UI
    login = true, -- Login/Logout
    update = true, -- Update command
    dashx = true, -- DashX prompts
    session = true, -- Session management
    lualine = true, -- Lualine integration
  },

  -- Keymap overrides
  -- Map action name to specific key string (e.g., send_selection = "<leader>x")
  -- or set to false to disable specific keymap even if feature is enabled
  keymaps = {},
}

-- ============================================================================
-- Command Definitions
-- ============================================================================

-- Definition of all available actions, their features, defaults, and behaviors
local actions = {
  -- Send Commands
  send_selection = {
    feature = "send",
    suffix = "sh",
    mode = "v",
    cmd = ":'<,'>AmpSendSelection<cr>",
    desc = "Send Selection (Content)",
  },
  send_selection_ref = {
    feature = "send",
    suffix = "sl",
    mode = "v",
    cmd = ":'<,'>AmpSendSelectionRef<cr>",
    desc = "Send Selection (Ref)",
  },
  send_buffer = {
    feature = "send",
    suffix = "sb",
    mode = "n",
    cmd = "<cmd>AmpSendBuffer<cr>",
    desc = "Send Buffer (Content)",
  },
  send_file_ref = {
    feature = "send",
    suffix = "sf",
    mode = "n",
    cmd = "<cmd>AmpSendFileRef<cr>",
    desc = "Send File (Ref)",
  },
  send_line_ref = {
    feature = "send",
    suffix = "sr",
    mode = "n",
    cmd = "<cmd>AmpSendLineRef<cr>",
    desc = "Send Line (Ref)",
  },

  -- Message
  send_message = {
    feature = "message",
    suffix = "sm",
    mode = "n",
    cmd = "<cmd>AmpSendMessage<cr>",
    desc = "Send Message UI",
  },

  -- Login/Account
  login = {
    feature = "login",
    suffix = "li",
    mode = "n",
    cmd = "<cmd>AmpLogin<cr>",
    desc = "Amp Login",
  },
  logout = {
    feature = "login",
    suffix = "lo",
    mode = "n",
    cmd = "<cmd>AmpLogout<cr>",
    desc = "Amp Logout",
  },

  -- Update
  update = {
    feature = "update",
    suffix = "u",
    mode = "n",
    cmd = "<cmd>AmpUpdate<cr>",
    desc = "Amp Update",
  },

  -- DashX
  dashx_list = {
    feature = "dashx",
    suffix = "pl",
    mode = "n",
    cmd = "<cmd>AmpDashX<cr>",
    desc = "DashX: List Prompts",
  },
  dashx_execute = {
    feature = "dashx",
    suffix = "px",
    mode = "n",
    cmd = "<cmd>AmpExecute<cr>",
    desc = "DashX: Execute Prompt",
  },

  -- Session
  session_new = {
    feature = "session",
    suffix = "in",
    mode = "n",
    cmd = "<cmd>AmpSession<cr>",
    desc = "Amp: New Session",
  },
  session_msg = {
    feature = "session",
    suffix = "im",
    mode = "n",
    cmd = "<cmd>AmpSessionWithMessage<cr>",
    desc = "Amp: Session with Message",
  },
}

-- Optional lualine integration module
M.lualine = require("amp_extras.lualine")

-- Active configuration (merged defaults + user config)
M.config = vim.deepcopy(defaults)

-- ============================================================================
-- Setup & Configuration
-- ============================================================================

--- Setup keymaps based on features and overrides
---@param config table Full configuration table
local function setup_keymaps(config)
  local prefix = "<leader>" .. (config.prefix or "a")
  local user_keymaps = config.keymaps or {}
  local features = config.features or {}

  for action_name, def in pairs(actions) do
    -- Check if feature is enabled
    if features[def.feature] then
      local lhs = nil

      -- Check for user override
      local override = user_keymaps[action_name]

      if override ~= nil then
        -- User explicitly configured this keymap
        if override ~= false then
          lhs = override
        end
      else
        -- Use default if not explicitly disabled
        lhs = prefix .. def.suffix
      end

      -- Apply keymap if valid
      if lhs then
        vim.keymap.set(def.mode, lhs, def.cmd, {
          noremap = true,
          silent = true,
          desc = def.desc,
        })
      end
    end
  end
end

--- Setup amp-extras plugin
---
--- Merges user configuration with defaults and initializes the plugin.
---
---@param opts table|nil User configuration options
---@return table Configuration
function M.setup(opts)
  -- Merge user config with defaults
  opts = opts or {}

  -- Handle legacy config structure migration if needed (simple check)
  if opts.keymaps and type(opts.keymaps.send_selection) == "boolean" and not opts.features then
    -- User is likely using old config style.
    -- We can try to map it, or just proceed and let the new defaults handle unset features.
    -- For now, we assume the user updates their config or we rely on defaults.
  end

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

  -- Setup Lualine integration if enabled
  if M.config.features.lualine then
    M.lualine.setup()
  end

  return M.config
end

--- Register UI commands (can also be called manually)
function M.register_ui_commands()
  local commands = require("amp_extras.commands")
  commands.register_commands()
end

return M
