--- AmpTab ghost text renderer
--- Renders inline suggestions via extmarks (like copilot.lua)
--- @module 'amp_extras.amptab.ghost'
local M = {}

---@type number Namespace for ghost text extmarks
M.ns = nil

---@type table|nil Currently displayed suggestion
M.current = nil

---@type number|nil Extmark ID for the ghost text
M.extmark_id = nil

---@type string|nil Hash of last shown suggestion (for dedup)
M.last_suggestion_hash = nil

---Highlight groups
M.hl_group = "Comment"
M.hl_group_indicator = "DiagnosticInfo"

---Initialize namespace
local function ensure_ns()
  if not M.ns then
    M.ns = vim.api.nvim_create_namespace("amptab_ghost")
  end
  return M.ns
end

---Clear any displayed ghost text
function M.dismiss()
  local ns = ensure_ns()
  if M.extmark_id then
    pcall(vim.api.nvim_buf_del_extmark, 0, ns, M.extmark_id)
    M.extmark_id = nil
  end
  -- Also clear any virtual lines
  vim.api.nvim_buf_clear_namespace(0, ns, 0, -1)
  M.current = nil
end

---Check if ghost text is currently visible
---@return boolean
function M.is_visible()
  return M.current ~= nil and M.extmark_id ~= nil
end

---Generate a simple hash for deduplication
---@param text string
---@param row number
---@return string
local function make_hash(text, row)
  return string.format("%d:%s", row, text:sub(1, 200))
end

---Check if a suggestion is duplicate of last shown
---@param completion table
---@return boolean
function M.is_duplicate(completion)
  if not M.last_suggestion_hash then
    return false
  end
  local hash = make_hash(completion.text or "", completion.cursor_row or 0)
  return hash == M.last_suggestion_hash
end

---Render ghost text for a completion
---@param completion table Completion data with text, full_text, range_start, range_end, cursor_row, cursor_col
---@param bufnr? number Buffer number (default: current)
function M.show(completion, bufnr)
  M.dismiss()

  bufnr = bufnr or vim.api.nvim_get_current_buf()
  local ns = ensure_ns()

  -- Validate buffer
  if not vim.api.nvim_buf_is_valid(bufnr) then
    return
  end

  local line_count = vim.api.nvim_buf_line_count(bufnr)
  local target_row = completion.cursor_row
  if target_row >= line_count then
    target_row = line_count - 1
  end
  if target_row < 0 then
    target_row = 0
  end

  -- Get current line content
  local current_line = vim.api.nvim_buf_get_lines(bufnr, target_row, target_row + 1, false)[1] or ""
  local cursor_col = math.min(completion.cursor_col or 0, #current_line)

  -- The display_text is the NEW portion to show
  local display_text = completion.text or ""
  if display_text == "" then
    return
  end

  -- Split into lines
  local lines = vim.split(display_text, "\n", { plain = true })
  if #lines == 0 then
    return
  end

  -- First line: inline virtual text at cursor position
  local first_line = lines[1]

  -- Text after cursor on current line (for positioning)
  local text_after_cursor = current_line:sub(cursor_col + 1)

  -- Determine virt_text_pos based on whether there's text after cursor
  local virt_text_pos = "inline"
  if #text_after_cursor > 0 then
    -- If there's text after cursor, use overlay or eol
    virt_text_pos = "eol"
  end

  local extmark_opts = {
    virt_text = {
      { "⚡ ", M.hl_group_indicator },
      { first_line, M.hl_group },
    },
    virt_text_pos = virt_text_pos,
    hl_mode = "combine",
    priority = 1000,
  }

  -- Add virtual lines for remaining lines
  if #lines > 1 then
    extmark_opts.virt_lines = {}
    for i = 2, #lines do
      table.insert(extmark_opts.virt_lines, {
        { lines[i], M.hl_group },
      })
    end
  end

  -- Set the extmark
  local ok, result =
    pcall(vim.api.nvim_buf_set_extmark, bufnr, ns, target_row, cursor_col, extmark_opts)
  if ok then
    M.extmark_id = result
    M.current = completion
    M.current.bufnr = bufnr
    -- Track for deduplication
    M.last_suggestion_hash = make_hash(display_text, target_row)
  end
end

---Accept the currently displayed ghost text
---@return boolean success
function M.accept()
  if not M.current then
    return false
  end

  local completion = M.current
  local bufnr = completion.bufnr or vim.api.nvim_get_current_buf()

  -- Validate buffer
  if not vim.api.nvim_buf_is_valid(bufnr) then
    M.dismiss()
    return false
  end

  -- Build the text edit
  local full_text = completion.full_text or completion.text
  local range = {
    start = {
      line = completion.range_start.row,
      character = completion.range_start.col,
    },
    ["end"] = {
      line = completion.range_end.row,
      character = completion.range_end.col,
    },
  }

  -- Clamp range to valid buffer bounds
  local line_count = vim.api.nvim_buf_line_count(bufnr)
  range.start.line = math.min(range.start.line, line_count - 1)
  range["end"].line = math.min(range["end"].line, line_count - 1)

  -- Clamp end character to line length
  local end_line_content = vim.api.nvim_buf_get_lines(
    bufnr,
    range["end"].line,
    range["end"].line + 1,
    false
  )[1] or ""
  range["end"].character = math.min(range["end"].character, #end_line_content)

  -- Clear ghost first
  M.dismiss()

  -- Schedule the edit to run outside of any restricted context
  vim.schedule(function()
    -- Create undo breakpoint
    vim.cmd("let &undolevels=&undolevels")

    -- Apply the edit using LSP utility
    local ok, err = pcall(function()
      vim.lsp.util.apply_text_edits({ { range = range, newText = full_text } }, bufnr, "utf-8")
    end)

    if not ok then
      vim.notify("[AmpTab] Failed to apply: " .. tostring(err), vim.log.levels.ERROR)
      return
    end

    -- Calculate new cursor position (end of inserted text)
    local new_lines = vim.split(full_text, "\n", { plain = true })
    local new_line_count = #new_lines
    local last_line_len = #new_lines[new_line_count]

    local new_cursor_row = range.start.line + new_line_count -- 1-indexed
    local new_cursor_col = last_line_len

    -- If single line, add to start column
    if new_line_count == 1 then
      new_cursor_col = range.start.character + last_line_len
    end

    -- Clamp to valid range
    local final_line_count = vim.api.nvim_buf_line_count(bufnr)
    if new_cursor_row > final_line_count then
      new_cursor_row = final_line_count
    end

    -- Set cursor position
    pcall(vim.api.nvim_win_set_cursor, 0, { new_cursor_row, new_cursor_col })
  end)

  return true
end

---Accept only the first line of the ghost text (partial accept)
---@return boolean success
function M.accept_line()
  if not M.current then
    return false
  end

  local completion = M.current
  local bufnr = completion.bufnr or vim.api.nvim_get_current_buf()

  local full_text = completion.full_text or completion.text
  local lines = vim.split(full_text, "\n", { plain = true })

  if #lines == 0 then
    M.dismiss()
    return false
  end

  -- Only apply first line
  local first_line = lines[1]

  -- Insert at cursor position
  local cursor = vim.api.nvim_win_get_cursor(0)
  local row = cursor[1] - 1
  local col = cursor[2]

  local current_line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
  local new_line = current_line:sub(1, col) .. first_line .. current_line:sub(col + 1)

  vim.api.nvim_buf_set_lines(bufnr, row, row + 1, false, { new_line })
  vim.api.nvim_win_set_cursor(0, { row + 1, col + #first_line })

  -- If there are more lines, update completion and re-show
  if #lines > 1 then
    local remaining = table.concat(lines, "\n", 2)
    completion.text = remaining
    completion.full_text = remaining
    completion.cursor_row = row
    completion.cursor_col = col + #first_line
    M.show(completion, bufnr)
    return true
  end

  M.dismiss()
  return true
end

---Accept only the next word of the ghost text (partial accept)
---@return boolean success
function M.accept_word()
  if not M.current then
    return false
  end

  local completion = M.current
  local bufnr = completion.bufnr or vim.api.nvim_get_current_buf()

  local display_text = completion.text or ""
  if display_text == "" then
    M.dismiss()
    return false
  end

  -- Extract first word (including trailing space)
  local word = display_text:match("^(%S+%s*)") or display_text:sub(1, 1)

  -- Insert at cursor position
  local cursor = vim.api.nvim_win_get_cursor(0)
  local row = cursor[1] - 1
  local col = cursor[2]

  local current_line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
  local new_line = current_line:sub(1, col) .. word .. current_line:sub(col + 1)

  vim.api.nvim_buf_set_lines(bufnr, row, row + 1, false, { new_line })
  vim.api.nvim_win_set_cursor(0, { row + 1, col + #word })

  -- If there's more text, update completion and re-show
  local remaining = display_text:sub(#word + 1)
  if remaining ~= "" then
    completion.text = remaining
    completion.cursor_row = row
    completion.cursor_col = col + #word
    M.show(completion, bufnr)
    return true
  end

  M.dismiss()
  return true
end

return M
