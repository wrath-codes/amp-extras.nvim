local M = {}

local ts_context = require("amp_extras.amptab.treesitter")
local enrichment = require("amp_extras.amptab.enrichment")

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
  -- Treesitter options
  use_treesitter = true,
  treesitter_max_lines = 100,
  prefer_function = true,
  -- Enrichment options
  use_enrichment = true,
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
---@field prefix_in_region string Text before cursor within editable region
---@field suffix_in_region string Text after cursor within editable region
---@field range_start {row: number, col: number} 0-indexed start of editable region
---@field range_end {row: number, col: number} 0-indexed end of editable region
---@field cursor_row number 0-indexed cursor row
---@field cursor_col number 0-indexed cursor column
---@field diagnostic_hint string|nil Diagnostic message at cursor
---@field ts_strategy string|nil Treesitter strategy used: "function"|"block"|"fallback"|nil
---@field ts_node_type string|nil Treesitter node type used for region

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
  local region_start_row, region_end_row
  local ts_region = nil
  local ts_strategy = nil

  -- Try treesitter-aware region selection
  if opts.use_treesitter and ts_context.is_available(bufnr) then
    ts_region = ts_context.get_editable_region(bufnr, cursor_row, cursor_col, {
      max_lines = opts.treesitter_max_lines,
      prefer_function = opts.prefer_function,
    })
    region_start_row = ts_region.start_row
    region_end_row = ts_region.end_row
    ts_strategy = ts_region.strategy
  else
    -- Fallback: fixed line counts (40 prefix + 360 suffix in lines)
    local rewrite_prefix_lines = math.ceil(opts.code_to_rewrite_prefix_tokens / 10)
    local rewrite_suffix_lines = math.ceil(opts.code_to_rewrite_suffix_tokens / 10)

    region_start_row = math.max(0, cursor_row - rewrite_prefix_lines)
    region_end_row = math.min(total_lines - 1, cursor_row + rewrite_suffix_lines)
  end

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

  -- Gather enrichment context (recent edits, viewed files, clipboard, lint errors)
  local enriched = nil
  if opts.use_enrichment then
    enriched = enrichment.get_all(bufnr, {
      range_start = region_start_row,
      range_end = region_end_row,
    })
  end

  -- Build FIM prompt matching official Amp extension format
  local prompt_parts = {}

  -- 1. Recently viewed snippets (before file)
  if enriched and enriched.recently_viewed then
    table.insert(
      prompt_parts,
      "Code snippets I have recently viewed, roughly from oldest to newest. Some may be irrelevant to the change:"
    )
    table.insert(prompt_parts, enriched.recently_viewed)
    table.insert(prompt_parts, "")
  end

  -- 2. Diff history (recent edits)
  if enriched and enriched.diff_history then
    table.insert(prompt_parts, "My recent edits, from oldest to newest:")
    table.insert(prompt_parts, enriched.diff_history)
    table.insert(prompt_parts, "")
  end

  -- 3. Lint errors
  if enriched and enriched.lint_errors then
    table.insert(prompt_parts, "Linter errors from the code that you will rewrite:")
    table.insert(prompt_parts, enriched.lint_errors)
    table.insert(prompt_parts, "")
  elseif diagnostic_hint then
    table.insert(prompt_parts, "Linter errors from the code that you will rewrite:")
    table.insert(prompt_parts, string.format("<lint_errors>\n%s\n</lint_errors>", diagnostic_hint))
    table.insert(prompt_parts, "")
  end

  -- 4. Recent copy (clipboard)
  if enriched and enriched.recent_copy then
    table.insert(prompt_parts, "Recently copied text. It may be irrelevant to the change:")
    table.insert(prompt_parts, enriched.recent_copy)
    table.insert(prompt_parts, "")
  end

  -- 5. The file currently open
  table.insert(prompt_parts, "The file currently open:")
  table.insert(prompt_parts, "<file>")

  -- 5a. Prefix before editable region
  table.insert(prompt_parts, prefix_before)

  -- 5b. Class context (header + __init__) if available - as inline comment
  local class_context = ts_region and ts_region.class_context or nil
  if class_context then
    table.insert(prompt_parts, "# CLASS CONTEXT (available self.* attributes):")
    table.insert(prompt_parts, class_context)
    table.insert(prompt_parts, "# END CLASS CONTEXT")
  end

  -- 5c. The editable region with cursor marker
  table.insert(prompt_parts, M.tokens.editable_region_start)
  table.insert(
    prompt_parts,
    code_to_rewrite_prefix .. M.tokens.user_cursor .. code_to_rewrite_suffix
  )
  table.insert(prompt_parts, M.tokens.editable_region_end)

  -- 5d. Suffix after editable region
  table.insert(prompt_parts, suffix_after)
  table.insert(prompt_parts, "</file>")

  local prompt = table.concat(prompt_parts, "\n")

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
    -- Treesitter metadata (for debugging/logging)
    ts_strategy = ts_strategy,
    ts_node_type = ts_region and ts_region.node_type or nil,
  }
end

return M
