-- Selection tracking and debounced notification
-- Based on amp.nvim's selection.lua implementation

local M = {}

-- Wrapper module reference (set by init.lua)
M.wrapper = nil

-- State tracking
M.state = {
  latest_selection = nil,
  debounce_timer = nil,
  debounce_ms = 10, -- 10ms is sweet spot per amp.nvim (100ms is too laggy)
}

--- Compare two selections to check if they changed
---@param sel1 table|nil First selection
---@param sel2 table|nil Second selection
---@return boolean changed True if selections are different
local function has_selection_changed(sel1, sel2)
  -- If either is nil, they're different
  if sel1 == nil or sel2 == nil then
    return true
  end

  -- Compare all fields
  return sel1.uri ~= sel2.uri
    or sel1.start_line ~= sel2.start_line
    or sel1.start_char ~= sel2.start_char
    or sel1.end_line ~= sel2.end_line
    or sel1.end_char ~= sel2.end_char
    or sel1.content ~= sel2.content
end

--- Get current cursor/selection state
---@return table|nil selection Current selection or nil if invalid
local function get_current_selection()
  local buf = vim.api.nvim_get_current_buf()
  local bufname = vim.api.nvim_buf_get_name(buf)

  -- Skip unnamed/scratch buffers
  if bufname == "" or not vim.startswith(bufname, "/") then
    return nil
  end

  -- Convert to file:// URI
  local uri = "file://" .. bufname

  -- Get mode
  local mode = vim.api.nvim_get_mode().mode

  local start_line, start_char, end_line, end_char, content

  -- Visual modes: v, V, CTRL-V (^V)
  if mode == "v" or mode == "V" or mode == "\22" then
    -- Get visual selection marks
    local start_pos = vim.fn.getpos("'<")
    local end_pos = vim.fn.getpos("'>")

    -- Check if marks are valid
    if start_pos[2] == 0 or end_pos[2] == 0 then
      -- Marks not set, fall back to cursor position
      local cursor = vim.api.nvim_win_get_cursor(0)
      start_line = cursor[1] - 1 -- Convert to 0-indexed
      start_char = cursor[2]
      end_line = start_line
      end_char = start_char
      content = ""
    else
      -- Convert from (1,0)-indexed to 0-indexed
      start_line = start_pos[2] - 1
      start_char = start_pos[3] - 1
      end_line = end_pos[2] - 1
      end_char = end_pos[3] - 1

      -- Get selected text
      local lines = vim.api.nvim_buf_get_text(
        buf,
        start_line,
        start_char,
        end_line,
        end_char + 1, -- end-exclusive
        {}
      )
      content = table.concat(lines, "\n")
    end
  else
    -- Normal mode - cursor position
    local cursor = vim.api.nvim_win_get_cursor(0)
    start_line = cursor[1] - 1 -- Convert to 0-indexed
    start_char = cursor[2]
    end_line = start_line
    end_char = start_char
    content = ""
  end

  return {
    uri = uri,
    start_line = start_line,
    start_char = start_char,
    end_line = end_line,
    end_char = end_char,
    content = content,
  }
end

--- Update and broadcast selection (called after debounce)
local function update_and_broadcast()
  local current = get_current_selection()

  -- Skip if invalid (unnamed buffer, etc.)
  if not current then
    return
  end

  -- Only broadcast if changed
  if has_selection_changed(M.state.latest_selection, current) then
    M.state.latest_selection = current

    -- Send via wrapper
    M.wrapper.send_selection_changed(
      current.uri,
      current.start_line,
      current.start_char,
      current.end_line,
      current.end_char,
      current.content
    )
  end
end

--- Debounced update - called on cursor events
function M.debounced_update()
  -- Cancel existing timer
  if M.state.debounce_timer then
    M.state.debounce_timer:stop()
    M.state.debounce_timer:close()
  end

  -- Schedule new update
  M.state.debounce_timer = vim.defer_fn(function()
    update_and_broadcast()
    M.state.debounce_timer = nil
  end, M.state.debounce_ms)
end

--- Setup selection tracking autocmds
function M.setup()
  local group = vim.api.nvim_create_augroup("AmpExtrasSelection", { clear = true })

  vim.api.nvim_create_autocmd({ "CursorMoved", "CursorMovedI" }, {
    group = group,
    callback = function()
      M.debounced_update()
    end,
  })
end

return M
