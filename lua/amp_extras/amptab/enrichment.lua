--- AmpTab context enrichment
--- Gathers additional context from Neovim environment for better completions
--- @module 'amp_extras.amptab.enrichment'
local M = {}

-- Recent edits storage (circular buffer)
M.recent_edits = {}
M.max_edits = 10

-- Recently viewed files
M.recent_files = {}
M.max_files = 5

---@class EnrichedContext
---@field diff_history string|nil Recent edits XML
---@field recently_viewed string|nil Recent file snippets XML
---@field recent_copy string|nil Clipboard content XML
---@field lint_errors string|nil Diagnostic errors XML

---Get recent clipboard/yank content
---@param max_chars? number Maximum characters to include
---@return string|nil
function M.get_recent_copy(max_chars)
  max_chars = max_chars or 500

  -- Try system clipboard first, then unnamed register
  local clipboard = vim.fn.getreg("+")
  if clipboard == "" then
    clipboard = vim.fn.getreg('"')
  end

  if clipboard == "" or #clipboard < 5 then
    return nil
  end

  -- Truncate if too long
  if #clipboard > max_chars then
    clipboard = clipboard:sub(1, max_chars) .. "..."
  end

  return string.format("<recent_copy>\n%s\n</recent_copy>", clipboard)
end

---Get lint/diagnostic errors for buffer
---@param bufnr number
---@param range_start? number Start row (0-indexed)
---@param range_end? number End row (0-indexed)
---@return string|nil
function M.get_lint_errors(bufnr, range_start, range_end)
  local diagnostics = vim.diagnostic.get(bufnr)
  if #diagnostics == 0 then
    return nil
  end

  -- Filter to range if provided
  if range_start and range_end then
    local filtered = {}
    for _, d in ipairs(diagnostics) do
      if d.lnum >= range_start and d.lnum <= range_end then
        table.insert(filtered, d)
      end
    end
    diagnostics = filtered
  end

  if #diagnostics == 0 then
    return nil
  end

  -- Sort by severity
  table.sort(diagnostics, function(a, b)
    return (a.severity or 4) < (b.severity or 4)
  end)

  -- Format as XML
  local lines = { "<lint_errors>" }
  local severity_names = { "ERROR", "WARN", "INFO", "HINT" }
  local seen = {}
  local count = 0

  for _, d in ipairs(diagnostics) do
    if count >= 5 then
      break
    end
    local key = d.lnum .. ":" .. (d.message or "")
    if not seen[key] then
      seen[key] = true
      local sev = severity_names[d.severity] or "UNKNOWN"
      table.insert(lines, string.format("Line %d [%s]: %s", d.lnum + 1, sev, d.message or ""))
      count = count + 1
    end
  end

  table.insert(lines, "</lint_errors>")
  return table.concat(lines, "\n")
end

---Track a buffer edit for diff history
---@param bufnr number
---@param old_lines string[]
---@param new_lines string[]
---@param start_row number
---@param end_row number
function M.track_edit(bufnr, old_lines, new_lines, start_row, end_row)
  local filename = vim.api.nvim_buf_get_name(bufnr)
  if filename == "" then
    return
  end

  -- Simple diff representation
  local edit = {
    file = vim.fn.fnamemodify(filename, ":t"),
    time = os.time(),
    old = table.concat(old_lines, "\n"),
    new = table.concat(new_lines, "\n"),
    line = start_row + 1,
  }

  table.insert(M.recent_edits, edit)

  -- Keep only recent edits
  while #M.recent_edits > M.max_edits do
    table.remove(M.recent_edits, 1)
  end
end

---Get diff history as XML
---@param max_edits? number
---@return string|nil
function M.get_diff_history(max_edits)
  max_edits = max_edits or 5

  if #M.recent_edits == 0 then
    return nil
  end

  local lines = { "<diff_history>" }
  local start_idx = math.max(1, #M.recent_edits - max_edits + 1)

  for i = start_idx, #M.recent_edits do
    local edit = M.recent_edits[i]
    table.insert(lines, string.format("<edit file=%q line=%d>", edit.file, edit.line))
    if edit.old ~= "" then
      table.insert(lines, "-" .. edit.old:gsub("\n", "\n-"))
    end
    if edit.new ~= "" then
      table.insert(lines, "+" .. edit.new:gsub("\n", "\n+"))
    end
    table.insert(lines, "</edit>")
  end

  table.insert(lines, "</diff_history>")
  return table.concat(lines, "\n")
end

---Track a file view
---@param bufnr number
function M.track_file_view(bufnr)
  local filename = vim.api.nvim_buf_get_name(bufnr)
  if filename == "" then
    return
  end

  -- Don't track if it's the current buffer being edited
  local ft = vim.bo[bufnr].filetype
  if ft == "" or ft == "TelescopePrompt" or ft == "nofile" then
    return
  end

  -- Remove if already in list
  for i, f in ipairs(M.recent_files) do
    if f.path == filename then
      table.remove(M.recent_files, i)
      break
    end
  end

  -- Get a snippet from the file (first 30 lines or around cursor)
  local lines = vim.api.nvim_buf_get_lines(bufnr, 0, 30, false)
  local snippet = table.concat(lines, "\n")

  table.insert(M.recent_files, {
    path = filename,
    name = vim.fn.fnamemodify(filename, ":t"),
    snippet = snippet,
    time = os.time(),
  })

  -- Keep only recent files
  while #M.recent_files > M.max_files do
    table.remove(M.recent_files, 1)
  end
end

---Get recently viewed snippets as XML
---@param current_file? string Exclude current file
---@param max_snippets? number
---@return string|nil
function M.get_recently_viewed(current_file, max_snippets)
  max_snippets = max_snippets or 3

  local files = {}
  for i = #M.recent_files, 1, -1 do
    local f = M.recent_files[i]
    if f.path ~= current_file and #files < max_snippets then
      table.insert(files, f)
    end
  end

  if #files == 0 then
    return nil
  end

  local lines = { "<recently_viewed_snippets>" }

  for _, f in ipairs(files) do
    table.insert(lines, string.format("<snippet file=%q>", f.name))
    -- Limit snippet size
    local snippet = f.snippet
    if #snippet > 1000 then
      snippet = snippet:sub(1, 1000) .. "\n..."
    end
    table.insert(lines, snippet)
    table.insert(lines, "</snippet>")
  end

  table.insert(lines, "</recently_viewed_snippets>")
  return table.concat(lines, "\n")
end

---Get all enrichment context
---@param bufnr number
---@param opts? {range_start?: number, range_end?: number}
---@return EnrichedContext
function M.get_all(bufnr, opts)
  opts = opts or {}
  local current_file = vim.api.nvim_buf_get_name(bufnr)

  return {
    diff_history = M.get_diff_history(5),
    recently_viewed = M.get_recently_viewed(current_file, 3),
    recent_copy = M.get_recent_copy(500),
    lint_errors = M.get_lint_errors(bufnr, opts.range_start, opts.range_end),
  }
end

---Setup autocmds for tracking
function M.setup()
  local group = vim.api.nvim_create_augroup("AmpTabEnrichment", { clear = true })

  -- Track file views
  vim.api.nvim_create_autocmd("BufEnter", {
    group = group,
    callback = function(ev)
      -- Delay to avoid tracking during quick buffer switches
      vim.defer_fn(function()
        if vim.api.nvim_buf_is_valid(ev.buf) then
          M.track_file_view(ev.buf)
        end
      end, 500)
    end,
  })

  -- Track edits using buffer attach
  vim.api.nvim_create_autocmd("BufEnter", {
    group = group,
    callback = function(ev)
      local bufnr = ev.buf
      if not vim.api.nvim_buf_is_valid(bufnr) then
        return
      end

      -- Attach to buffer for change tracking
      vim.api.nvim_buf_attach(bufnr, false, {
        on_lines = function(_, buf, _, first_line, last_line, new_last_line)
          -- Only track significant edits (not single char changes)
          if math.abs(new_last_line - last_line) > 0 or last_line - first_line > 1 then
            local new_lines = vim.api.nvim_buf_get_lines(buf, first_line, new_last_line, false)
            M.track_edit(buf, {}, new_lines, first_line, new_last_line)
          end
          return false -- Keep attached
        end,
      })
    end,
  })
end

return M
