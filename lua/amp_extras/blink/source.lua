local M = {}

---@return string[]
function M:get_trigger_characters()
  return { "@" }
end

---@param context blink.cmp.Context|nil
---@return boolean
function M:enabled(context)
  local bufnr = context and context.bufnr or vim.api.nvim_get_current_buf()
  return vim.b[bufnr].amp_message_context == true
end

---@param context blink.cmp.Context
function M:get_completions(context, callback)
  local line = context.line
  local col = context.cursor[2]

  if not line then
    callback()
    return
  end

  local trigger_pos = nil
  local query = ""

  for i = col, 1, -1 do
    local char = line:sub(i, i)
    if char == "@" then
      trigger_pos = i
      break
    end
    if char:match("%s") then
      break
    end
  end

  if not trigger_pos then
    callback()
    return
  end

  query = line:sub(trigger_pos + 1, col)

  local ok, core = pcall(require, "amp_extras_core")
  if not ok then
    callback()
    return
  end

  local items = {}

  local files = core.autocomplete("file", query)
  if files then
    for _, file in ipairs(files) do
      table.insert(items, {
        label = file,
        kind = 17,
        insertText = "@" .. file,
        sortText = "0_" .. file,
        filterText = "@" .. file,
        labelDetails = { detail = "File" },
        documentation = {
          kind = "markdown",
          value = string.format("**File**: `%s`", file),
        },
      })
    end
  end

  local threads = core.autocomplete("thread", query)
  if threads then
    for _, thread_str in ipairs(threads) do
      local id, title = thread_str:match("^(T%-[%w-]+):%s*(.+)$")
      if not id then
        id = thread_str
        title = ""
      end

      table.insert(items, {
        label = title ~= "" and title or id,
        kind = 18,
        insertText = "@" .. id,
        sortText = "1_" .. (title ~= "" and title or id),
        filterText = "@" .. id .. " " .. title,
        labelDetails = { detail = id, description = "Thread" },
        documentation = {
          kind = "markdown",
          value = string.format("**Thread**\n\n**ID**: `%s`\n**Title**: %s", id, title),
        },
      })
    end
  end

  callback {
    is_incomplete_forward = false,
    is_incomplete_backward = false,
    items = items,
  }
end

function M.new()
  return setmetatable({}, { __index = M })
end

return M
