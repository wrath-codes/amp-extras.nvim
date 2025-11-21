local n = require("nui-components")
local api = require("amp_extras.commands.dashx.api")

local M = {}

---@class DashXFormProps
---@field mode "create"|"edit"|"bulk_tags"
---@field prompt Prompt?
---@field on_success fun(prompt: Prompt|boolean|string[]|nil)

---Show the Create/Edit Prompt Form
---@param props DashXFormProps
function M.show(props)
  local is_edit = props.mode == "edit"
  local is_bulk_tags = props.mode == "bulk_tags"
  local prompt = props.prompt or { title = "", content = "", tags = nil, description = "" }

  -- Convert tags array to comma-separated string
  local tags_str = ""
  if prompt.tags and #prompt.tags > 0 then
    tags_str = table.concat(prompt.tags, ", ")
  end

  local renderer = n.create_renderer({
    width = 80,
    height = is_bulk_tags and 10 or 30,
  })

  local window_style = {
    highlight = {
      FloatBorder = "DiagnosticError",
      FloatTitle = "DiagnosticError",
    },
  }

  local form_component = n.form({
    id = "prompt_form",
    submit_key = "<C-s>", -- Ctrl+s to save
    on_submit = function(is_valid)
      if not is_valid then
        vim.notify("Please fix the errors in the form", vim.log.levels.ERROR)
        return
      end

      local component = renderer:get_component_by_id("prompt_form")
      if not component then return end
      
      local tags_comp = renderer:get_component_by_id("tags_input")
      if not tags_comp then return end
      local tags_val = tags_comp:get_current_value()
      
      -- Process tags
      local tags = {}
      for tag in string.gmatch(tags_val, "([^,]+)") do
        local clean_tag = vim.trim(tag)
        clean_tag = string.gsub(clean_tag, "[\r\n]", " ")
        if clean_tag ~= "" then
          table.insert(tags, clean_tag)
        end
      end
      if #tags == 0 then tags = nil end

      if is_bulk_tags then
        renderer:close()
        if props.on_success then
          props.on_success(tags)
        end
        return
      end

      local title_comp = renderer:get_component_by_id("title_input")
      local description_comp = renderer:get_component_by_id("description_input")
      local content_comp = renderer:get_component_by_id("content_input")

      local title = title_comp:get_current_value()
      local description = description_comp and description_comp:get_current_value()
      local content = content_comp:get_current_value()

      if description and vim.trim(description) == "" then
        description = nil
      end

      -- Call API
      local success, result
      if is_edit then
        success, result = pcall(api.update_prompt, prompt.id, title, description, content, tags)
      else
        success, result = pcall(api.create_prompt, title, description, content, tags)
      end

      if success then
        vim.notify("Prompt " .. (is_edit and "updated" or "created"), vim.log.levels.INFO)
        renderer:close()
        if props.on_success then
          props.on_success(result)
        end
      else
        vim.notify("Error: " .. tostring(result), vim.log.levels.ERROR)
      end
    end
  }, is_bulk_tags and n.rows(
    n.text_input({
      id = "tags_input",
      border_label = "Tags (comma separated)",
      placeholder = "rust, logic, testing",
      value = tags_str,
      window = window_style,
      autofocus = true, -- Autofocus tags in bulk mode
      on_mount = function(component)
        -- Map <CR> to submit in normal/insert mode to prevent newlines and auto-save
        if component.bufnr then
          vim.keymap.set({"n", "i"}, "<CR>", function()
            local form = renderer:get_component_by_id("prompt_form")
            if form then form:submit() end
          end, { buffer = component.bufnr, nowait = true })
        end
      end,
    }),
    n.gap(1),
    n.paragraph({
      lines = "Press <Enter> or <C-s> to save tags for all selected items, <Esc> to cancel",
      align = "center",
      is_focusable = false,
      border_label = "Actions",
      window = window_style,
    })
  ) or n.rows(
    n.text_input({
      id = "title_input",
      border_label = "Title",
      placeholder = "e.g., Refactor Code",
      value = prompt.title,
      validate = n.validator.min_length(3),
      autofocus = true,
      window = window_style,
    }),
    n.gap(1),
    n.text_input({
      id = "description_input",
      border_label = "Description",
      placeholder = "Brief description (optional)",
      value = prompt.description,
      max_lines = 1,
      window = window_style,
    }),
    n.gap(1),
    n.text_input({
      id = "tags_input",
      border_label = "Tags (comma separated)",
      placeholder = "rust, logic, testing",
      value = tags_str,
      window = window_style,
    }),
    n.gap(1),
    n.text_input({
      id = "content_input",
      border_label = "Prompt Content",
      placeholder = "Enter your prompt here...",
      value = prompt.content,
      flex = 1,
      window = window_style,
    }),
    n.gap(1),
    n.paragraph({
      lines = "Press <C-s> to save, <Esc> to cancel",
      align = "center",
      is_focusable = false,
      border_label = "Actions",
      window = window_style,
    })
  ))

  renderer:render(form_component)
end

return M
