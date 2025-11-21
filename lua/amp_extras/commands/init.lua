-- Command registry for amp-extras
local M = {}

-- Load UI commands
M.ui = {
  send_message_box = require("amp_extras.commands.ui").send_message,
  login = require("amp_extras.commands.ui").login,
}

-- Load send commands
local send = require("amp_extras.commands.send")
local logout = require("amp_extras.commands.logout")
local update = require("amp_extras.commands.update")

--- Register all commands as Neovim user commands
function M.register_commands()
  -- UI Commands
  vim.api.nvim_create_user_command("AmpLogin", function()
    M.ui.login.command()
  end, {
    desc = "Amp: Log in to Amp",
  })

  vim.api.nvim_create_user_command("AmpLogout", function()
    logout.command()
  end, {
    desc = "Amp: Log out from Amp",
  })

  vim.api.nvim_create_user_command("AmpUpdate", function()
    update.command()
  end, {
    desc = "Amp: Update Amp CLI",
  })

  vim.api.nvim_create_user_command("AmpSendMessage", function()
    M.ui.send_message_box.command()
  end, {
    desc = "Amp: Open send message UI",
  })

  -- Send commands (Lua, through sourcegraph/amp.nvim)
  vim.api.nvim_create_user_command("AmpSendFileRef", function()
    send.send_file_ref()
  end, {
    desc = "Amp: Send file reference to Amp prompt (@file.rs)",
  })

  vim.api.nvim_create_user_command("AmpSendLineRef", function()
    send.send_line_ref()
  end, {
    desc = "Amp: Send current line reference to Amp prompt (@file.rs#L10)",
  })

  vim.api.nvim_create_user_command("AmpSendBuffer", function()
    send.send_buffer()
  end, {
    desc = "Amp: Send entire buffer content to Amp prompt",
  })

  vim.api.nvim_create_user_command("AmpSendSelection", function(cmd_opts)
    send.send_selection(cmd_opts)
  end, {
    range = true,
    desc = "Amp: Send selected text to Amp prompt",
  })

  vim.api.nvim_create_user_command("AmpSendSelectionRef", function(cmd_opts)
    send.send_selection_ref(cmd_opts)
  end, {
    range = true,
    desc = "Amp: Send file reference with selected line range to Amp prompt (@file.rs#L10-L20)",
  })

  require("amp_extras.commands.dashx").setup()
end

return M
