-- Visible files tracking and debounced notification
-- Based on amp.nvim's visible_files.lua implementation

local M = {}

-- Wrapper module reference (set by init.lua)
M.wrapper = nil

-- State tracking
M.state = {
  latest_uris = nil,
  debounce_timer = nil,
  debounce_ms = 10, -- 10ms debounce
}

--- Compare two URI lists to check if they changed
---@param uris1 table|nil First URI list
---@param uris2 table|nil Second URI list
---@return boolean changed True if lists are different
local function has_uris_changed(uris1, uris2)
  if uris1 == nil or uris2 == nil then
    return true
  end
  
  if #uris1 ~= #uris2 then
    return true
  end
  
  -- Sort and compare
  local sorted1 = vim.fn.sort(vim.deepcopy(uris1))
  local sorted2 = vim.fn.sort(vim.deepcopy(uris2))
  
  for i = 1, #sorted1 do
    if sorted1[i] ~= sorted2[i] then
      return true
    end
  end
  
  return false
end

--- Get currently visible file URIs
---@return table uris List of file:// URIs
local function get_visible_uris()
  local uris = {}
  local seen = {}
  
  -- Get all windows
  for _, win in ipairs(vim.api.nvim_list_wins()) do
    local buf = vim.api.nvim_win_get_buf(win)
    local bufname = vim.api.nvim_buf_get_name(buf)
    
    -- Only include absolute paths (skip unnamed/scratch buffers)
    if bufname ~= "" and vim.startswith(bufname, "/") then
      -- Deduplicate
      if not seen[bufname] then
        seen[bufname] = true
        table.insert(uris, "file://" .. bufname)
      end
    end
  end
  
  return uris
end

--- Update and broadcast visible files (called after debounce)
local function update_and_broadcast()
  local current_uris = get_visible_uris()
  
  -- Only broadcast if changed
  if has_uris_changed(M.state.latest_uris, current_uris) then
    M.state.latest_uris = current_uris
    
    -- Send via wrapper
    M.wrapper.send_visible_files_changed(current_uris)
  end
end

--- Debounced update - called on buffer/window events
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

--- Setup visible files tracking autocmds
function M.setup()
  local group = vim.api.nvim_create_augroup("AmpExtrasVisibleFiles", { clear = true })
  
  vim.api.nvim_create_autocmd({ "BufEnter", "WinEnter" }, {
    group = group,
    callback = function()
      M.debounced_update()
    end,
  })
end

return M
