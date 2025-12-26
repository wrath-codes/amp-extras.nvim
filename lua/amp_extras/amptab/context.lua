local M = {}

-- Special tokens for FIM prompt
M.tokens = {
  editable_region_start = "<|editable_region_start|>",
  editable_region_end = "<|editable_region_end|>",
  user_cursor = "<|user_cursor_is_here|>",
}

-- Default token limits (increased for better multi-line completions)
M.defaults = {
  prefix_tokens = 1500,
  suffix_tokens = 1500,
  code_to_rewrite_prefix_tokens = 100,
  code_to_rewrite_suffix_tokens = 900,
}

---Approximate token count (rough: 4 chars per token)
---@param text string
---@return number
local function estimate_tokens(text)
  return math.ceil(#text / 4)
end

---Truncate text to approximate token limit from the end (keep suffix)
---@param text string
---@param max_tokens number
---@return string
local function truncate_start(text, max_tokens)
  local max_chars = max_tokens * 4
  if #text <= max_chars then
    return text
  end
  return text:sub(-max_chars)
end

---Truncate text to approximate token limit from the start (keep prefix)
---@param text string
---@param max_tokens number
---@return string
local function truncate_end(text, max_tokens)
  local max_chars = max_tokens * 4
  if #text <= max_chars then
    return text
  end
  return text:sub(1, max_chars)
end

---@class AmpTabContext
---@field prompt string The FIM prompt to send
---@field code_to_rewrite string The code in the editable region (for prediction)
---@field range_start {row: number, col: number} 0-indexed start of editable region
---@field range_end {row: number, col: number} 0-indexed end of editable region

---Get diagnostic at cursor position
---@param bufnr number
---@param row number 0-indexed
---@return string|nil diagnostic message
local function get_diagnostic_at_cursor(bufnr, row)
  local diagnostics = vim.diagnostic.get(bufnr, { lnum = row })
  if #diagnostics > 0 then
    -- Return most severe diagnostic
    table.sort(diagnostics, function(a, b)
      return a.severity < b.severity
    end)
    return diagnostics[1].message
  end
  return nil
end

---Build context for AmpTab completion
---@param bufnr? number Buffer number (default: current)
---@param opts? table Token limit overrides and diagnostic_hint
---@return AmpTabContext
function M.build(bufnr, opts)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  opts = vim.tbl_extend("force", M.defaults, opts or {})

  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_row = cursor[1] - 1 -- 0-indexed
  local cursor_col = cursor[2]

  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, -1, false)
  local total_lines = #lines

  -- Calculate editable region bounds
  -- We want ~400 tokens around cursor (40 prefix + 360 suffix in lines)
  local rewrite_prefix_lines = math.ceil(opts.code_to_rewrite_prefix_tokens / 10)
  local rewrite_suffix_lines = math.ceil(opts.code_to_rewrite_suffix_tokens / 10)

  local region_start_row = math.max(0, cursor_row - rewrite_prefix_lines)
  local region_end_row = math.min(total_lines - 1, cursor_row + rewrite_suffix_lines)

  -- Build the different sections
  local prefix_lines = {}
  for i = 1, region_start_row do
    table.insert(prefix_lines, lines[i])
  end
  local prefix_before = truncate_start(table.concat(prefix_lines, "\n"), opts.prefix_tokens)

  local suffix_lines = {}
  for i = region_end_row + 2, total_lines do
    table.insert(suffix_lines, lines[i])
  end
  local suffix_after = truncate_end(table.concat(suffix_lines, "\n"), opts.suffix_tokens)

  -- Build editable region content
  local rewrite_lines = {}
  for i = region_start_row + 1, region_end_row + 1 do
    table.insert(rewrite_lines, lines[i] or "")
  end

  -- Split at cursor position within the editable region
  local cursor_line_in_region = cursor_row - region_start_row
  local rewrite_prefix_parts = {}
  local rewrite_suffix_parts = {}

  for i, line in ipairs(rewrite_lines) do
    local line_idx = i - 1 -- 0-indexed within region
    if line_idx < cursor_line_in_region then
      table.insert(rewrite_prefix_parts, line)
    elseif line_idx == cursor_line_in_region then
      -- Split this line at cursor
      table.insert(rewrite_prefix_parts, line:sub(1, cursor_col))
      table.insert(rewrite_suffix_parts, line:sub(cursor_col + 1))
    else
      table.insert(rewrite_suffix_parts, line)
    end
  end

  local code_to_rewrite_prefix = table.concat(rewrite_prefix_parts, "\n")
  local code_to_rewrite_suffix = table.concat(rewrite_suffix_parts, "\n")
  local code_to_rewrite = code_to_rewrite_prefix .. code_to_rewrite_suffix

  -- Check for diagnostic at cursor to include as hint
  local diagnostic_hint = opts.diagnostic_hint or get_diagnostic_at_cursor(bufnr, cursor_row)
  
  -- Build FIM prompt
  -- If there's a diagnostic, add it as a comment hint before the editable region
  local diagnostic_comment = ""
  if diagnostic_hint then
    -- Format as a comment based on file type
    local ft = vim.bo[bufnr].filetype
    local comment_prefix = "# "  -- Default to Python/shell style
    if ft == "lua" then
      comment_prefix = "-- "
    elseif ft == "javascript" or ft == "typescript" or ft == "rust" or ft == "go" or ft == "c" or ft == "cpp" or ft == "java" then
      comment_prefix = "// "
    end
    diagnostic_comment = "\n" .. comment_prefix .. "FIX: " .. diagnostic_hint:gsub("\n", " ") .. "\n"
  end
  
  local prompt = prefix_before
    .. diagnostic_comment
    .. "\n" .. M.tokens.editable_region_start .. "\n"
    .. code_to_rewrite_prefix
    .. M.tokens.user_cursor
    .. code_to_rewrite_suffix
    .. "\n" .. M.tokens.editable_region_end .. "\n"
    .. suffix_after

  return {
    prompt = prompt,
    code_to_rewrite = code_to_rewrite,
    -- Track prefix/suffix separately for accurate diff extraction
    prefix_in_region = code_to_rewrite_prefix,
    suffix_in_region = code_to_rewrite_suffix,
    range_start = { row = region_start_row, col = 0 },
    range_end = { row = region_end_row, col = #(lines[region_end_row + 1] or "") },
    -- Cursor position for insertion
    cursor_row = cursor_row,
    cursor_col = cursor_col,
    diagnostic_hint = diagnostic_hint,
  }
end

return M
