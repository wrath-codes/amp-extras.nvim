-- Command registry for amp-extras
local M = {}

-- Load UI commands
M.ui = {
  send_message_box = require("amp_extras.commands.ui").send_message,
}

--- Register all commands as Neovim user commands
function M.register_commands()
  -- UI Commands
  vim.api.nvim_create_user_command("AmpSendMessage", function()
    M.ui.send_message_box.command()
  end, {
    desc = "Amp: Open send message UI",
  })

  -- Server Status Command
  vim.api.nvim_create_user_command("AmpStatus", function()
    local amp = require("amp_extras")
    local status = amp.get_status()

    if status.running then
      local port = status.port or 0
      local clients = status.clients or 0
      local client_text = clients == 1 and "client" or "clients"
      local msg = string.format("Amp server running on :%d (%d %s)", port, clients, client_text)
      vim.notify(msg, vim.log.levels.INFO, { title = "Amp" })
    else
      vim.notify("Amp server stopped", vim.log.levels.WARN, { title = "Amp" })
    end
  end, {
    desc = "Amp: Show server status",
  })
end

return M
