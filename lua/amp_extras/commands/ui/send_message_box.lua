local Text = require("nui.text")
local TextInput = require("nui-components.text-input")
local fn = require("nui-components.utils.fn")

local Prompt = TextInput:extend("Prompt")

function Prompt:init(props, popup_options)
  Prompt.super.init(
    self,
    fn.merge({
      on_submit = fn.ignore,
      prefix = "",
      submit_key = "<C-j>",  -- Dummy key (custom mappings handle Enter/Shift+Enter)
      autoresize = true,
      max_lines = 10,  -- Allow up to 10 lines
    }, props),
    fn.deep_merge({
      buf_options = {
        buftype = "prompt",
      },
      win_options = {
        wrap = true,  -- Enable text wrapping at window width
        linebreak = true,  -- Break at word boundaries, not mid-word
      },
    }, popup_options)
  )
end

function Prompt:prop_types()
  return fn.merge(Prompt.super.prop_types(self), {
    on_submit = "function",
    prefix = "string",
    submit_key = { "table", "string" },
  })
end

function Prompt:on_mount()
  local props = self:get_props()

  local function is_nui_text(value)
    return value.class and value.class.name == "NuiText"
  end

  self._private.prefix = is_nui_text(props.prefix) and props.prefix
    or Text(props.prefix, self:hl_group("Prefix"))
  
  -- Set empty prompt to avoid repeating prefix on new lines
  vim.fn.prompt_setprompt(self.bufnr, "")

  -- Call parent on_mount to set up change listener with autoresize signal
  Prompt.super.on_mount(self)
end

function Prompt:on_update()
  local mode = vim.fn.mode()
  local current_winid = vim.api.nvim_get_current_win()

  if not (current_winid == self.winid and mode == "i") then
    vim.schedule(function()
      if self:is_first_render() then
        local value = table.concat(self:get_lines(), "")
        vim.api.nvim_feedkeys(value, "n", true)
        vim.api.nvim_win_set_cursor(self.winid, { 1, #value })
      end
    end)
  end
end

function Prompt:mappings()
  local props = self:get_props()

  return {
    {
      mode = "i",
      key = "<CR>",  -- Enter to submit
      handler = function()
        props.on_submit(self:get_current_value())
      end,
    },
    {
      mode = "i",
      key = "<S-CR>",  -- Shift+Enter to insert newline
      handler = function()
        -- Insert newline and stay in insert mode
        vim.api.nvim_feedkeys("\n", "n", true)
      end,
    },
  }
end

local M = {}

-- Load FFI module at module level (not inside function)
local ffi = require("amp_extras.ffi")

-- Send message to agent
M.command = function()
  local n = require("nui-components")

  local renderer = n.create_renderer({
    width = 60,
    height = 3,  -- Start small, will grow dynamically
  })

  -- Debounce timer to prevent excessive renderer resizes
  local resize_timer = nil
  local last_height = 3

  local body = function()
    return Prompt({
      autofocus = true,
      autoresize = true,  -- Let TextInput manage component size internally
      prefix = " 󱐋 ",
      placeholder = "Type your message to Amp...",
      border_label = {
        text = "Amp Message",
        icon = "󰄾",
        edge = "top",
        align = "left",
      },
      on_change = function(value)
        -- Simpler approach: grow based on character count + actual newlines
        -- Every ~50 chars = 1 line, plus explicit newlines
        local char_count = #value
        local newline_count = select(2, value:gsub("\n", "\n"))
        
        -- Estimate: 1 line per 50 chars, plus 1 for each newline
        local estimated_lines = math.ceil(char_count / 50) + newline_count
        local new_height = math.min(estimated_lines + 4, 14)  -- +4 for overhead
        
        -- Immediately resize without debounce to be more responsive
        if new_height ~= last_height then
          last_height = new_height
          renderer:set_size({ height = new_height })
        end
      end,
      on_submit = function(value)
        -- Send the value as-is (preserves newlines from Shift+Enter)
        ffi.send_user_message(value)
        renderer:close()
      end,
      window = {
        relative = "editor",
        focusable = true,
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