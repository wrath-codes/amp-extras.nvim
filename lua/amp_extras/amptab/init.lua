local M = {}

local context = require("amp_extras.amptab.context")
local client = require("amp_extras.amptab.client")
local ghost = require("amp_extras.amptab.ghost")

M.config = {
  enabled = true,
  debug = false,
  debounce_ms = 150,
  preload = true,
  auto_trigger = true, -- Trigger on CursorHoldI
  updatetime = 400, -- CursorHold delay (ms)
  default_keymaps = true, -- Set default ga* keymaps
  token_limits = {
    prefix_tokens = 1500,
    suffix_tokens = 1500,
    code_to_rewrite_prefix_tokens = 100,
    code_to_rewrite_suffix_tokens = 900,
    -- Treesitter-aware context (SAFIM)
    use_treesitter = true,
    treesitter_max_lines = 100,
    prefer_function = true,
    -- Context enrichment (recent edits, viewed files, clipboard, lint errors)
    use_enrichment = true,
  },
}

-- Expose ghost module
M.ghost = ghost

---@class AmpTabCompletion
---@field text string The completion text
---@field range_start {row: number, col: number}
---@field range_end {row: number, col: number}

-- Special tokens to strip from output
local SPECIAL_TOKENS = {
  "<|editable_region_start|>",
  "<|editable_region_end|>",
  "<|user_cursor_is_here|>",
}

---Strip special tokens from text
---@param text string
---@return string
local function strip_tokens(text)
  for _, token in ipairs(SPECIAL_TOKENS) do
    text = text:gsub(vim.pesc(token), "")
  end
  return text
end

---@type fun()|nil Current cancel function
local current_cancel = nil

---Cancel any in-flight request
function M.cancel()
  if current_cancel then
    current_cancel()
    current_cancel = nil
  end
end

---Request a completion
---@param callback fun(completion: AmpTabCompletion|nil, err: string|nil)
---@param opts? table
---@return fun() cancel
function M.complete(callback, opts)
  M.cancel()

  local ctx = context.build(nil, M.config.token_limits)

  local full_text = ""

  current_cancel = client.complete({
    prompt = ctx.prompt,
    code_to_rewrite = ctx.code_to_rewrite,
    max_tokens = 1024,
  }, function(chunk)
    full_text = full_text .. chunk
  end, function(final_text)
    current_cancel = nil

    if M.config.debug then
      vim.notify("[AmpTab] Raw response:\n" .. final_text, vim.log.levels.DEBUG)
    end

    local cleaned_text = strip_tokens(final_text)
    -- Only trim trailing whitespace, preserve leading indentation
    cleaned_text = cleaned_text:gsub("%s+$", "")
    -- Remove leading newlines only (not spaces)
    cleaned_text = cleaned_text:gsub("^\n+", "")

    if M.config.debug then
      vim.notify("[AmpTab] Cleaned response:\n" .. cleaned_text, vim.log.levels.DEBUG)
      vim.notify(
        string.format(
          "[AmpTab] Range: (%d,%d) to (%d,%d)",
          ctx.range_start.row,
          ctx.range_start.col,
          ctx.range_end.row,
          ctx.range_end.col
        ),
        vim.log.levels.DEBUG
      )
    end

    if cleaned_text == "" then
      callback(nil, nil)
      return
    end

    -- Clamp range to valid buffer bounds
    local bufnr = vim.api.nvim_get_current_buf()
    local line_count = vim.api.nvim_buf_line_count(bufnr)
    local clamped_start = {
      row = math.min(ctx.range_start.row, line_count - 1),
      col = ctx.range_start.col,
    }
    local clamped_end = {
      row = math.min(ctx.range_end.row, line_count - 1),
      col = ctx.range_end.col,
    }
    -- Clamp end col to line length
    local end_line = vim.api.nvim_buf_get_lines(bufnr, clamped_end.row, clamped_end.row + 1, false)[1]
      or ""
    clamped_end.col = math.min(clamped_end.col, #end_line)

    -- The model returns the full editable region with completion filled in.
    -- Pass prefix/suffix info so source can extract just the inserted text.
    callback({
      text = cleaned_text,
      prefix_in_region = ctx.prefix_in_region,
      suffix_in_region = ctx.suffix_in_region,
      range_start = clamped_start,
      range_end = clamped_end,
      cursor_row = ctx.cursor_row,
      cursor_col = ctx.cursor_col,
    }, nil)
  end, function(err)
    current_cancel = nil
    callback(nil, err)
  end)

  return function()
    M.cancel()
  end
end

-- Cache and preloader modules
M.cache = require("amp_extras.amptab.cache")
M.preloader = require("amp_extras.amptab.preloader")
M.treesitter = require("amp_extras.amptab.treesitter")
M.enrichment = require("amp_extras.amptab.enrichment")

---Dismiss ghost text
function M.dismiss()
  ghost.dismiss()
end

---Accept ghost text and auto-trigger next suggestion (hot streak)
---@return boolean success
function M.accept()
  local success = ghost.accept()
  if success then
    -- Auto-trigger next suggestion after a short delay (hot streak)
    vim.defer_fn(function()
      M.trigger()
    end, 100)
  end
  return success
end

---Accept only the current line
---@return boolean success
function M.accept_line()
  return ghost.accept_line()
end

---Accept only the next word
---@return boolean success
function M.accept_word()
  return ghost.accept_word()
end

---Check if ghost is visible
---@return boolean
function M.is_visible()
  return ghost.is_visible()
end

---Trigger a completion at current cursor and show as ghost text
---@param callback? fun(success: boolean)
function M.trigger(callback)
  M.cancel()

  local ctx = context.build(nil, M.config.token_limits)
  local full_text = ""

  if M.config.debug then
    local strategy = ctx.ts_strategy or "line-count"
    local node_type = ctx.ts_node_type or "n/a"
    vim.notify(
      string.format(
        "[AmpTab] Context: strategy=%s, node=%s, range=[%d,%d]",
        strategy,
        node_type,
        ctx.range_start.row,
        ctx.range_end.row
      ),
      vim.log.levels.DEBUG
    )
  end

  current_cancel = client.complete({
    prompt = ctx.prompt,
    code_to_rewrite = ctx.code_to_rewrite,
    max_tokens = 1024,
  }, function(chunk)
    full_text = full_text .. chunk
  end, function(final_text)
    current_cancel = nil

    local cleaned_text = strip_tokens(final_text)
    cleaned_text = cleaned_text:gsub("%s+$", ""):gsub("^\n+", "")

    if cleaned_text == "" then
      if callback then
        callback(false)
      end
      return
    end

    -- Extract display text (new portion only)
    local prefix = ctx.prefix_in_region or ""
    local suffix = ctx.suffix_in_region or ""

    local suffix_match_len = 0
    local min_suffix_len = math.min(#suffix, #cleaned_text)
    for i = 1, min_suffix_len do
      if suffix:byte(#suffix - i + 1) == cleaned_text:byte(#cleaned_text - i + 1) then
        suffix_match_len = i
      else
        break
      end
    end

    local prefix_match_len = 0
    local max_prefix_check = #cleaned_text - suffix_match_len
    local min_prefix_len = math.min(#prefix, max_prefix_check)
    for i = 1, min_prefix_len do
      if prefix:byte(i) == cleaned_text:byte(i) then
        prefix_match_len = i
      else
        break
      end
    end

    local display_text =
      cleaned_text:sub(prefix_match_len + 1, #cleaned_text - suffix_match_len):gsub("%s+$", "")

    if M.config.debug then
      vim.notify(
        string.format(
          "[AmpTab] Extract: prefix_match=%d, suffix_match=%d, display_len=%d",
          prefix_match_len,
          suffix_match_len,
          #display_text
        ),
        vim.log.levels.DEBUG
      )
    end

    if display_text == "" then
      if callback then
        callback(false)
      end
      return
    end

    -- Clamp range to valid buffer bounds
    local bufnr = vim.api.nvim_get_current_buf()
    local line_count = vim.api.nvim_buf_line_count(bufnr)
    local clamped_start = {
      row = math.min(ctx.range_start.row, line_count - 1),
      col = ctx.range_start.col,
    }
    local clamped_end = {
      row = math.min(ctx.range_end.row, line_count - 1),
      col = ctx.range_end.col,
    }
    local end_line = vim.api.nvim_buf_get_lines(bufnr, clamped_end.row, clamped_end.row + 1, false)[1]
      or ""
    clamped_end.col = math.min(clamped_end.col, #end_line)

    local completion = {
      text = display_text,
      full_text = cleaned_text,
      prefix_in_region = ctx.prefix_in_region,
      suffix_in_region = ctx.suffix_in_region,
      range_start = clamped_start,
      range_end = clamped_end,
      cursor_row = ctx.cursor_row,
      cursor_col = ctx.cursor_col,
    }

    -- Show ghost text
    ghost.show(completion)

    -- Add to cache
    M.cache.add(completion, "cursor")

    if callback then
      callback(true)
    end
  end, function(err)
    current_cancel = nil
    if M.config.debug then
      vim.notify("[AmpTab] " .. err, vim.log.levels.WARN)
    end
    if callback then
      callback(false)
    end
  end)
end

---Setup AmpTab
---@param opts? table
function M.setup(opts)
  if opts then
    M.config = vim.tbl_deep_extend("force", M.config, opts)
  end

  -- Set updatetime for CursorHold
  if M.config.auto_trigger and M.config.updatetime then
    vim.o.updatetime = M.config.updatetime
  end

  -- Setup enrichment tracking (recent edits, viewed files)
  if M.config.token_limits.use_enrichment then
    M.enrichment.setup()
  end

  local group = vim.api.nvim_create_augroup("AmpTab", { clear = true })

  -- Auto-trigger on CursorHoldI
  if M.config.auto_trigger then
    vim.api.nvim_create_autocmd("CursorHoldI", {
      group = group,
      callback = function()
        if not M.config.enabled then
          return
        end
        -- Don't trigger if ghost is already visible
        if ghost.is_visible() then
          return
        end
        -- Check excluded filetypes
        local ft = vim.bo.filetype
        local excluded = { "TelescopePrompt", "nofile", "help", "qf", "" }
        for _, ex in ipairs(excluded) do
          if ft == ex then
            return
          end
        end
        M.trigger()
      end,
    })
  end

  -- Dismiss ghost on cursor move (in insert mode)
  vim.api.nvim_create_autocmd("CursorMovedI", {
    group = group,
    callback = function()
      -- Only dismiss if we moved significantly (not just accepting)
      if ghost.is_visible() then
        ghost.dismiss()
      end
    end,
  })

  -- Dismiss ghost on leaving insert mode
  vim.api.nvim_create_autocmd("InsertLeave", {
    group = group,
    callback = function()
      ghost.dismiss()
    end,
  })

  -- Dismiss ghost on buffer change
  vim.api.nvim_create_autocmd("BufLeave", {
    group = group,
    callback = function()
      ghost.dismiss()
    end,
  })

  -- Register commands
  vim.api.nvim_create_user_command("AmpTabAccept", function()
    M.accept()
  end, { desc = "Accept AmpTab ghost text" })

  vim.api.nvim_create_user_command("AmpTabAcceptLine", function()
    M.accept_line()
  end, { desc = "Accept current line of AmpTab ghost text" })

  vim.api.nvim_create_user_command("AmpTabAcceptWord", function()
    M.accept_word()
  end, { desc = "Accept next word of AmpTab ghost text" })

  vim.api.nvim_create_user_command("AmpTabDismiss", function()
    M.dismiss()
  end, { desc = "Dismiss AmpTab ghost text" })

  vim.api.nvim_create_user_command("AmpTabTrigger", function()
    M.trigger()
  end, { desc = "Manually trigger AmpTab completion" })

  vim.api.nvim_create_user_command("AmpTabClear", function()
    M.cache.clear()
    ghost.dismiss()
    vim.notify("[AmpTab] Cache cleared", vim.log.levels.INFO)
  end, { desc = "Clear AmpTab cache" })

  -- Enable preloader if configured
  if M.config.preload then
    M.preloader.enable()
  end

  -- Set default keymaps
  if M.config.default_keymaps then
    local map = vim.keymap.set

    -- Normal mode: trigger and accept
    map("n", "gaa", function()
      M.accept()
    end, { desc = "AmpTab: Accept suggestion" })
    map("n", "gad", function()
      M.dismiss()
    end, { desc = "AmpTab: Dismiss suggestion" })
    map("n", "gat", function()
      M.trigger()
    end, { desc = "AmpTab: Trigger suggestion" })

    -- Insert mode: accept with Tab (if ghost visible), else fallback
    map("i", "<Tab>", function()
      if M.is_visible() then
        M.accept()
        return ""
      end
      return "<Tab>"
    end, { expr = true, desc = "AmpTab: Accept or Tab" })

    -- C-a to accept in both insert and normal mode
    map({ "i", "n" }, "<C-a>", function()
      if M.is_visible() then
        M.accept()
      end
    end, { desc = "AmpTab: Accept suggestion" })

    -- Partial accepts
    map("i", "<C-l>", function()
      if M.is_visible() then
        M.accept_line()
      end
    end, { desc = "AmpTab: Accept line" })

    map("i", "<C-Right>", function()
      if M.is_visible() then
        M.accept_word()
        return ""
      end
      return "<C-Right>"
    end, { expr = true, desc = "AmpTab: Accept word" })

    -- Dismiss with Esc doesn't need special handling - InsertLeave autocmd does it
  end
end

return M
