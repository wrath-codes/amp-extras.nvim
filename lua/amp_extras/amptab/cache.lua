--- AmpTab completion cache for storing and navigating multiple completions
--- @module 'amp_extras.amptab.cache'
local M = {}

---@class CachedCompletion
---@field id string Unique cache ID
---@field text string The completion text
---@field full_text string Full editable region replacement
---@field range_start {row: number, col: number} Start of editable region
---@field range_end {row: number, col: number} End of editable region
---@field cursor_row number Original cursor row (0-indexed)
---@field cursor_col number Original cursor col
---@field bufnr number Buffer number
---@field timestamp number When this was cached
---@field source string Where this came from: "cursor" | "diagnostic" | "preload"

---@type CachedCompletion[]
M.items = {}

---@type number Max items to keep in cache
M.max_items = 20

---@type string|nil Currently displayed completion ID
M.current_id = nil

---@type number|nil Current index in items being shown
M.current_index = nil

---Generate a unique ID
---@return string
local function generate_id()
  return string.format("%x-%x", os.time(), math.random(0, 0xFFFF))
end

---Add a completion to the cache
---@param completion table Completion data from source.lua
---@param source? string Source type (default: "cursor")
---@return string id The cache ID
function M.add(completion, source)
  local id = generate_id()
  
  local item = {
    id = id,
    text = completion.text,
    full_text = completion.full_text or completion.text,
    range_start = completion.range_start,
    range_end = completion.range_end,
    cursor_row = completion.cursor_row,
    cursor_col = completion.cursor_col,
    bufnr = vim.api.nvim_get_current_buf(),
    timestamp = vim.loop.now(),
    source = source or "cursor",
  }
  
  -- Remove old items from same buffer/position (dedup)
  M.items = vim.tbl_filter(function(existing)
    if existing.bufnr ~= item.bufnr then
      return true
    end
    -- Keep if different position (more than 5 lines apart)
    local line_diff = math.abs(existing.cursor_row - item.cursor_row)
    return line_diff > 5
  end, M.items)
  
  table.insert(M.items, item)
  
  -- Trim cache if too large
  while #M.items > M.max_items do
    table.remove(M.items, 1)
  end
  
  M.current_id = id
  M.current_index = #M.items
  
  return id
end

---Get completion by ID
---@param id string
---@return CachedCompletion|nil
function M.get(id)
  for _, item in ipairs(M.items) do
    if item.id == id then
      return item
    end
  end
  return nil
end

---Get completions for current buffer sorted by distance from cursor
---@return CachedCompletion[]
function M.get_for_current_buffer()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_row = cursor[1] - 1
  
  local items = vim.tbl_filter(function(item)
    return item.bufnr == bufnr
  end, M.items)
  
  -- Sort by distance from cursor
  table.sort(items, function(a, b)
    local dist_a = math.abs(a.cursor_row - cursor_row)
    local dist_b = math.abs(b.cursor_row - cursor_row)
    return dist_a < dist_b
  end)
  
  return items
end

---Get next completion after current cursor position
---@return CachedCompletion|nil, number|nil index
function M.get_next()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_row = cursor[1] - 1
  
  -- Find completions after cursor, sorted by distance
  local candidates = {}
  for i, item in ipairs(M.items) do
    if item.bufnr == bufnr and item.cursor_row > cursor_row then
      table.insert(candidates, { item = item, index = i })
    end
  end
  
  if #candidates == 0 then
    return nil, nil
  end
  
  -- Sort by row (closest first)
  table.sort(candidates, function(a, b)
    return a.item.cursor_row < b.item.cursor_row
  end)
  
  return candidates[1].item, candidates[1].index
end

---Get previous completion before current cursor position
---@return CachedCompletion|nil, number|nil index
function M.get_prev()
  local bufnr = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  local cursor_row = cursor[1] - 1
  
  -- Find completions before cursor, sorted by distance
  local candidates = {}
  for i, item in ipairs(M.items) do
    if item.bufnr == bufnr and item.cursor_row < cursor_row then
      table.insert(candidates, { item = item, index = i })
    end
  end
  
  if #candidates == 0 then
    return nil, nil
  end
  
  -- Sort by row (closest first, descending)
  table.sort(candidates, function(a, b)
    return a.item.cursor_row > b.item.cursor_row
  end)
  
  return candidates[1].item, candidates[1].index
end

---Remove a completion from cache
---@param id string
function M.remove(id)
  M.items = vim.tbl_filter(function(item)
    return item.id ~= id
  end, M.items)
  
  if M.current_id == id then
    M.current_id = nil
    M.current_index = nil
  end
end

---Clear all cached items for a buffer
---@param bufnr? number Buffer number (default: current)
function M.clear_buffer(bufnr)
  bufnr = bufnr or vim.api.nvim_get_current_buf()
  M.items = vim.tbl_filter(function(item)
    return item.bufnr ~= bufnr
  end, M.items)
  M.current_id = nil
  M.current_index = nil
end

---Clear entire cache
function M.clear()
  M.items = {}
  M.current_id = nil
  M.current_index = nil
end

---Get cache status
---@return table
function M.status()
  local bufnr = vim.api.nvim_get_current_buf()
  local buffer_items = vim.tbl_filter(function(item)
    return item.bufnr == bufnr
  end, M.items)
  
  return {
    total = #M.items,
    current_buffer = #buffer_items,
    current_id = M.current_id,
  }
end

return M
