--- AmpTab navigator for jumping between diagnostics and triggering AI fixes
--- @module 'amp_extras.amptab.navigator'
local M = {}

local cache = require("amp_extras.amptab.cache")
local ghost = require("amp_extras.amptab.ghost")

---@type CachedCompletion|nil Currently previewing
M.previewing = nil

---@type table<string, number> Recently visited diagnostic locations (key -> timestamp)
M.visited = {}

---@type number How long to remember visited locations (ms)
M.visited_ttl = 30000 -- 30 seconds

---Mark a diagnostic location as visited
---@param lnum number
---@param bufnr number
local function mark_visited(lnum, bufnr)
  local key = string.format("%d:%d", bufnr, lnum)
  M.visited[key] = vim.loop.now()
end

---Check if a diagnostic was recently visited
---@param lnum number
---@param bufnr number
---@return boolean
local function was_visited(lnum, bufnr)
  local key = string.format("%d:%d", bufnr, lnum)
  local ts = M.visited[key]
  if not ts then
    return false
  end
  if vim.loop.now() - ts > M.visited_ttl then
    M.visited[key] = nil
    return false
  end
  return true
end

---Clear visited history
function M.clear_visited()
  M.visited = {}
end

---Clear any preview
function M.clear_preview()
  ghost.dismiss()
  M.previewing = nil
end

---Show preview at a cached completion's location using ghost text
---@param item CachedCompletion
function M.show_preview(item)
  M.clear_preview()

  local bufnr = vim.api.nvim_get_current_buf()

  if item.bufnr ~= bufnr then
    return
  end

  -- Show using ghost module
  ghost.show(item, bufnr)
  M.previewing = item
end

---Get diagnostics sorted by line, filtered by severity
---@param bufnr number
---@return table[] diagnostics
local function get_sorted_diagnostics(bufnr)
  local diagnostics = vim.diagnostic.get(bufnr)

  -- Filter to errors and warnings only
  diagnostics = vim.tbl_filter(function(d)
    return d.severity == vim.diagnostic.severity.ERROR or d.severity == vim.diagnostic.severity.WARN
  end, diagnostics)

  -- Sort by line number
  table.sort(diagnostics, function(a, b)
    if a.lnum ~= b.lnum then
      return a.lnum < b.lnum
    end
    return a.col < b.col
  end)

  return diagnostics
end

---Jump to next diagnostic and trigger AI fix suggestion
---@return boolean success
function M.next()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_row = cursor[1] - 1 -- 0-indexed

  local diagnostics = get_sorted_diagnostics(bufnr)

  -- Find first diagnostic after cursor (skip recently visited)
  local target = nil
  local skipped = 0
  for _, d in ipairs(diagnostics) do
    if d.lnum > cursor_row then
      if not was_visited(d.lnum, bufnr) then
        target = d
        break
      else
        skipped = skipped + 1
      end
    end
  end

  -- Wrap around to first unvisited diagnostic if none found below
  if not target then
    for _, d in ipairs(diagnostics) do
      if not was_visited(d.lnum, bufnr) then
        target = d
        if skipped > 0 or cursor_row > 0 then
          vim.notify("[AmpTab] Wrapped to first diagnostic", vim.log.levels.INFO)
        end
        break
      end
    end
  end

  -- If all visited, clear history and try again
  if not target and #diagnostics > 0 then
    M.clear_visited()
    target = diagnostics[1]
    vim.notify("[AmpTab] All diagnostics visited, starting over", vim.log.levels.INFO)
  end

  if not target then
    vim.notify("[AmpTab] No diagnostics found", vim.log.levels.INFO)
    return false
  end

  -- Mark as visited
  mark_visited(target.lnum, bufnr)

  -- Move cursor to diagnostic
  vim.api.nvim_win_set_cursor(0, { target.lnum + 1, target.col })

  -- Show diagnostic message briefly
  vim.notify(string.format("[AmpTab] %s", target.message:sub(1, 80)), vim.log.levels.INFO)

  -- Trigger fresh completion at this location
  vim.schedule(function()
    local amptab = require("amp_extras.amptab")
    amptab.trigger()
  end)

  return true
end

---Jump to previous diagnostic and trigger AI fix suggestion
---@return boolean success
function M.prev()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_row = cursor[1] - 1 -- 0-indexed

  local diagnostics = get_sorted_diagnostics(bufnr)

  -- Find last diagnostic before cursor (skip recently visited)
  local target = nil
  local skipped = 0
  for i = #diagnostics, 1, -1 do
    local d = diagnostics[i]
    if d.lnum < cursor_row then
      if not was_visited(d.lnum, bufnr) then
        target = d
        break
      else
        skipped = skipped + 1
      end
    end
  end

  -- Wrap around to last unvisited diagnostic if none found above
  if not target then
    for i = #diagnostics, 1, -1 do
      local d = diagnostics[i]
      if not was_visited(d.lnum, bufnr) then
        target = d
        if skipped > 0 or cursor_row < vim.api.nvim_buf_line_count(bufnr) then
          vim.notify("[AmpTab] Wrapped to last diagnostic", vim.log.levels.INFO)
        end
        break
      end
    end
  end

  -- If all visited, clear history and try again
  if not target and #diagnostics > 0 then
    M.clear_visited()
    target = diagnostics[#diagnostics]
    vim.notify("[AmpTab] All diagnostics visited, starting over", vim.log.levels.INFO)
  end

  if not target then
    vim.notify("[AmpTab] No diagnostics found", vim.log.levels.INFO)
    return false
  end

  -- Mark as visited
  mark_visited(target.lnum, bufnr)

  -- Move cursor to diagnostic
  vim.api.nvim_win_set_cursor(0, { target.lnum + 1, target.col })

  -- Show diagnostic message briefly
  vim.notify(string.format("[AmpTab] %s", target.message:sub(1, 80)), vim.log.levels.INFO)

  -- Trigger fresh completion at this location
  vim.schedule(function()
    local amptab = require("amp_extras.amptab")
    amptab.trigger()
  end)

  return true
end

---Accept the currently previewing completion
---@return boolean success
function M.accept()
  if not M.previewing then
    -- Try ghost module directly
    return ghost.accept()
  end

  local item = M.previewing
  local bufnr = vim.api.nvim_get_current_buf()

  if item.bufnr ~= bufnr then
    vim.notify("[AmpTab] Completion is for a different buffer", vim.log.levels.WARN)
    return false
  end

  -- Use ghost.accept() which handles the edit properly
  local success = ghost.accept()

  if success then
    -- Remove from cache
    cache.remove(item.id)
    M.previewing = nil

    -- Check for next completion and show it
    local next_item = cache.get_next()
    if next_item then
      vim.schedule(function()
        M.show_preview(next_item)
      end)
    end
  end

  return success
end

---Reject the currently previewing completion
function M.reject()
  if M.previewing then
    cache.remove(M.previewing.id)
  end
  M.clear_preview()
end

---Show status of cached completions
function M.status()
  local status = cache.status()
  local msg =
    string.format("[AmpTab] Cache: %d items (%d in buffer)", status.total, status.current_buffer)
  vim.notify(msg, vim.log.levels.INFO)
end

return M
