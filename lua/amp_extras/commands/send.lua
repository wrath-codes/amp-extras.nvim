local M = {}

local function get_amp_message()
  local ok, amp_message = pcall(require, "amp.message")
  if not ok then
    vim.notify(
      "amp-extras.nvim: sourcegraph/amp.nvim (module 'amp.message') not found. " ..
      "Install https://github.com/sourcegraph/amp.nvim and run require('amp').setup(...).",
      vim.log.levels.ERROR,
      { title = "Amp Extras" }
    )
    return nil
  end
  return amp_message
end

function M.send_file_ref()
  local amp_message = get_amp_message()
  if not amp_message then return end

  local bufname = vim.api.nvim_buf_get_name(0)
  if bufname == "" then
    vim.notify("Current buffer has no filename", vim.log.levels.WARN, { title = "Amp Extras" })
    return
  end

  local relative_path = vim.fn.fnamemodify(bufname, ":.")
  local ref = "@" .. relative_path

  amp_message.send_to_prompt(ref)
end

function M.send_line_ref()
  local amp_message = get_amp_message()
  if not amp_message then return end

  local bufname = vim.api.nvim_buf_get_name(0)
  if bufname == "" then
    vim.notify("Current buffer has no filename", vim.log.levels.WARN, { title = "Amp Extras" })
    return
  end

  local relative_path = vim.fn.fnamemodify(bufname, ":.")
  local line = vim.api.nvim_win_get_cursor(0)[1]
  local ref = string.format("@%s#L%d", relative_path, line)

  amp_message.send_to_prompt(ref)
end

function M.send_buffer()
  local amp_message = get_amp_message()
  if not amp_message then return end

  local buf = vim.api.nvim_get_current_buf()
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local content = table.concat(lines, "\n")

  amp_message.send_to_prompt(content)
end

function M.send_selection(cmd_opts)
  local amp_message = get_amp_message()
  if not amp_message then return end

  -- Use marks for exact visual selection (character-accurate)
  local start_pos = vim.api.nvim_buf_get_mark(0, "<")
  local end_pos = vim.api.nvim_buf_get_mark(0, ">")
  if start_pos[1] == 0 or end_pos[1] == 0 then
    -- Fallback: line range from user command
    local lines = vim.api.nvim_buf_get_lines(0, cmd_opts.line1 - 1, cmd_opts.line2, false)
    local text = table.concat(lines, "\n")
    amp_message.send_to_prompt(text)
    return
  end

  local lines = vim.api.nvim_buf_get_text(
    0,
    start_pos[1] - 1, start_pos[2],
    end_pos[1] - 1, end_pos[2] + 1,
    {}
  )
  local text = table.concat(lines, "\n")
  amp_message.send_to_prompt(text)
end

function M.send_selection_ref(cmd_opts)
  local amp_message = get_amp_message()
  if not amp_message then return end

  local bufname = vim.api.nvim_buf_get_name(0)
  if bufname == "" then
    vim.notify("Current buffer has no filename", vim.log.levels.WARN, { title = "Amp Extras" })
    return
  end

  local relative_path = vim.fn.fnamemodify(bufname, ":.")
  local ref = "@" .. relative_path
  local line1, line2 = cmd_opts.line1, cmd_opts.line2

  if line1 ~= line2 then
    ref = ref .. "#L" .. line1 .. "-" .. line2
  elseif line1 > 1 then
    ref = ref .. "#L" .. line1
  end

  amp_message.send_to_prompt(ref)
end

return M
