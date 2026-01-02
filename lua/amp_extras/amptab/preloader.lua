--- AmpTab preloader for proactively fetching completions at diagnostic locations
--- Integrates with amp.nvim's diagnostic tracking
--- @module 'amp_extras.amptab.preloader'
local M = {}

local cache = require("amp_extras.amptab.cache")
local context = require("amp_extras.amptab.context")
local client = require("amp_extras.amptab.client")

---@type boolean Whether preloader is active
M.enabled = false

---@type number|nil Timer for debounced preloading
M.timer = nil

---@type table<string, boolean> URIs currently being preloaded
M.in_flight = {}

---@type number Debounce delay in ms
M.debounce_ms = 2000

---@type number Max preloads per buffer
M.max_preloads_per_buffer = 3

---@type number Namespace for diagnostic indicators
M.ns = nil

-- Special tokens to strip
local SPECIAL_TOKENS = {
  "<|editable_region_start|>",
  "<|editable_region_end|>",
  "<|user_cursor_is_here|>",
}

local function strip_tokens(text)
  for _, token in ipairs(SPECIAL_TOKENS) do
    text = text:gsub(vim.pesc(token), "")
  end
  return text
end

---Try to get amp.nvim diagnostics module
---@return table|nil
local function get_amp_diagnostics()
  local ok, diag = pcall(require, "amp.diagnostics")
  if ok then
    return diag
  end
  return nil
end

---Get diagnostics for current buffer using amp.nvim or fallback to vim.diagnostic
---@param bufnr number
---@return table[] List of {row, col, severity, message}
local function get_buffer_diagnostics(bufnr)
  local results = {}

  -- Try amp.nvim first
  local amp_diag = get_amp_diagnostics()
  if amp_diag then
    local buf_name = vim.api.nvim_buf_get_name(bufnr)
    if buf_name and buf_name ~= "" then
      local entries = amp_diag.get_diagnostics(buf_name)
      for _, entry in ipairs(entries) do
        for _, d in ipairs(entry.diagnostics or {}) do
          table.insert(results, {
            row = d.range.startLine,
            col = d.range.startCharacter,
            severity = d.severity,
            message = d.description,
          })
        end
      end
    end
  end

  -- Fallback to vim.diagnostic if no amp.nvim results
  if #results == 0 then
    local diagnostics = vim.diagnostic.get(bufnr)
    for _, d in ipairs(diagnostics) do
      table.insert(results, {
        row = d.lnum,
        col = d.col,
        severity = d.severity == vim.diagnostic.severity.ERROR and "error"
          or d.severity == vim.diagnostic.severity.WARN and "warning"
          or "info",
        message = d.message,
      })
    end
  end

  -- Sort by severity (errors first) then by line
  table.sort(results, function(a, b)
    local sev_order = { error = 1, warning = 2, info = 3, hint = 4 }
    local sev_a = sev_order[a.severity] or 5
    local sev_b = sev_order[b.severity] or 5
    if sev_a ~= sev_b then
      return sev_a < sev_b
    end
    return a.row < b.row
  end)

  return results
end

---Preload completion at a specific position
---@param bufnr number
---@param row number 0-indexed
---@param col number
---@param source string
local function preload_at_position(bufnr, row, col, source)
  local key = string.format("%d:%d:%d", bufnr, row, col)

  -- Skip if already in flight
  if M.in_flight[key] then
    return
  end

  -- Check if we already have a cached completion near this location
  local existing = cache.get_for_current_buffer()
  for _, item in ipairs(existing) do
    if math.abs(item.cursor_row - row) <= 3 then
      return -- Already have something nearby
    end
  end

  M.in_flight[key] = true

  -- Temporarily move cursor to build context (restore after)
  local win = vim.api.nvim_get_current_win()
  local orig_cursor = vim.api.nvim_win_get_cursor(win)

  -- Set cursor to diagnostic position
  local line_count = vim.api.nvim_buf_line_count(bufnr)
  local target_row = math.min(row + 1, line_count) -- 1-indexed
  local line = vim.api.nvim_buf_get_lines(bufnr, row, row + 1, false)[1] or ""
  local target_col = math.min(col, #line)

  pcall(vim.api.nvim_win_set_cursor, win, { target_row, target_col })

  -- Build context at this position
  local amptab = require("amp_extras.amptab")
  local ctx = context.build(bufnr, amptab.config.token_limits)

  -- Restore cursor
  pcall(vim.api.nvim_win_set_cursor, win, orig_cursor)

  -- Make the request
  client.complete(
    {
      prompt = ctx.prompt,
      code_to_rewrite = ctx.code_to_rewrite,
      max_tokens = 1024,
    },
    function(_) end, -- on_chunk (ignore streaming)
    function(final_text)
      M.in_flight[key] = nil

      if not final_text or final_text == "" then
        return
      end

      local cleaned = strip_tokens(final_text):gsub("%s+$", ""):gsub("^\n+", "")
      if cleaned == "" then
        return
      end

      -- Extract display text
      local prefix = ctx.prefix_in_region or ""
      local suffix = ctx.suffix_in_region or ""

      local suffix_match_len = 0
      local min_suffix_len = math.min(#suffix, #cleaned)
      for i = 1, min_suffix_len do
        if suffix:byte(#suffix - i + 1) == cleaned:byte(#cleaned - i + 1) then
          suffix_match_len = i
        else
          break
        end
      end

      local prefix_match_len = 0
      local max_prefix_check = #cleaned - suffix_match_len
      local min_prefix_len = math.min(#prefix, max_prefix_check)
      for i = 1, min_prefix_len do
        if prefix:byte(i) == cleaned:byte(i) then
          prefix_match_len = i
        else
          break
        end
      end

      local display_text =
        cleaned:sub(prefix_match_len + 1, #cleaned - suffix_match_len):gsub("%s+$", "")

      if display_text == "" then
        return
      end

      -- Cache it
      cache.add({
        text = display_text,
        full_text = cleaned,
        range_start = ctx.range_start,
        range_end = ctx.range_end,
        cursor_row = row,
        cursor_col = col,
      }, source)

      if amptab.config.debug then
        vim.schedule(function()
          vim.notify(
            string.format("[AmpTab] Preloaded completion at line %d (%s)", row + 1, source),
            vim.log.levels.DEBUG
          )
        end)
      end
    end,
    function(_)
      M.in_flight[key] = nil
    end
  )
end

---Preload completions for diagnostics in current buffer
function M.preload_diagnostics()
  if not M.enabled then
    return
  end

  local bufnr = vim.api.nvim_get_current_buf()
  local diagnostics = get_buffer_diagnostics(bufnr)

  -- Only preload top N diagnostics (prioritizing errors)
  local count = 0
  for _, d in ipairs(diagnostics) do
    if count >= M.max_preloads_per_buffer then
      break
    end

    -- Only preload errors and warnings
    if d.severity == "error" or d.severity == "warning" then
      preload_at_position(bufnr, d.row, d.col, "diagnostic")
      count = count + 1
    end
  end
end

---Schedule a debounced preload
function M.schedule_preload()
  if not M.enabled then
    return
  end

  if M.timer then
    vim.fn.timer_stop(M.timer)
  end

  M.timer = vim.fn.timer_start(M.debounce_ms, function()
    vim.schedule(function()
      M.preload_diagnostics()
    end)
  end)
end

---Enable the preloader
function M.enable()
  if M.enabled then
    return
  end

  M.enabled = true
  M.ns = vim.api.nvim_create_namespace("amptab_preloader")

  -- Create autocommands
  local group = vim.api.nvim_create_augroup("AmpTabPreloader", { clear = true })

  -- Preload on diagnostic changes
  vim.api.nvim_create_autocmd("DiagnosticChanged", {
    group = group,
    callback = function()
      M.schedule_preload()
    end,
  })

  -- Preload when entering a buffer
  vim.api.nvim_create_autocmd("BufEnter", {
    group = group,
    callback = function()
      -- Delay to let diagnostics populate
      vim.defer_fn(function()
        M.schedule_preload()
      end, 500)
    end,
  })

  -- Preload after being idle
  vim.api.nvim_create_autocmd({ "CursorHold", "CursorHoldI" }, {
    group = group,
    callback = function()
      M.schedule_preload()
    end,
  })
end

---Disable the preloader
function M.disable()
  M.enabled = false

  if M.timer then
    vim.fn.timer_stop(M.timer)
    M.timer = nil
  end

  vim.api.nvim_clear_autocmds({ group = "AmpTabPreloader" })
end

---Toggle the preloader
function M.toggle()
  if M.enabled then
    M.disable()
    vim.notify("[AmpTab] Preloader disabled", vim.log.levels.INFO)
  else
    M.enable()
    vim.notify("[AmpTab] Preloader enabled", vim.log.levels.INFO)
  end
end

return M
