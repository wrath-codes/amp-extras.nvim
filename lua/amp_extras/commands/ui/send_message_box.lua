local M = {}

-- Load FFI module at module level (not inside function)
local ffi = require("amp_extras.ffi")

-- Send message to agent
M.command = function()
  local n = require("nui-components")

  local renderer = n.create_renderer({
    width = 60,
    height = 10,
  })

  local body = function()
    return n.prompt({
      autofocus = true,
      prefix = " 󱐋 ",
      placeholder = "Type your message to Amp...",
      border_label = {
        text = "Amp Message",
        icon = "󰄾",
        edge = "top",
        align = "left",
      },
      on_submit = function(value)
        ffi.send_user_message(value)
        renderer:close()
      end,
      window = {
        highlight = {
          Normal = "Normal",
          FloatBorder = "DiagnosticError",
          FloatTitle = "DiagnosticError",
          NormalFloat = "NormalFloat",
          NuiComponentsPromptPrefix = "DiagnosticError",
        },
      },
    })
  end

  renderer:render(body)
end

return M
