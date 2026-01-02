local M = {}

M.defaults = {
  base_url = "https://ampcode.com",
  model = "amp-tab-long-suggestion-model-instruct",
  temperature = 0.1,
}

-- User prompt prefix (from official Amp extension)
M.user_prompt_prefix =
  [[Help me finish a coding change. You will see snippets from current open files in my editor, files I have recently viewed, the file I am editing, then a history of my recent codebase changes, then current compiler and linter errors, content I copied from my codebase. You will then rewrite the code between the <|editable_region_start|> and <|editable_region_end|> tags, to match what you think I would do next in the codebase. <|user_cursor_is_here|> indicates the position of the cursor in the the current file. Note: I might have stopped in the middle of typing.

]]

---@class AmpTabRequest
---@field prompt string FIM prompt
---@field code_to_rewrite string Code for prediction
---@field max_tokens? number

---@class AmpTabResponse
---@field text string Generated completion text
---@field done boolean Whether streaming is complete

---Parse SSE data line
---@param line string
---@return string|nil content
local function parse_sse_chunk(line)
  if not line or line == "" then
    return nil
  end

  -- SSE format: "data: {...}"
  local data = line:match("^data:%s*(.+)$")
  if not data then
    return nil
  end

  if data == "[DONE]" then
    return nil
  end

  local ok, parsed = pcall(vim.json.decode, data)
  if not ok or not parsed then
    return nil
  end

  -- OpenAI format: choices[1].text or choices[1].delta.content
  if parsed.choices and parsed.choices[1] then
    local choice = parsed.choices[1]
    if choice.text then
      return choice.text
    elseif choice.delta and choice.delta.content then
      return choice.delta.content
    end
  end

  return nil
end

---Get API key from environment
---@return string|nil
local function get_api_key()
  return os.getenv("AMP_API_KEY")
end

---Make streaming request to AmpTab API
---@param request AmpTabRequest
---@param on_chunk fun(text: string) Called for each chunk
---@param on_done fun(full_text: string) Called when complete
---@param on_error fun(err: string) Called on error
---@return fun() cancel Cancel function
function M.complete(request, on_chunk, on_done, on_error)
  local api_key = get_api_key()
  if not api_key then
    on_error("AMP_API_KEY not set")
    return function() end
  end

  local url = M.defaults.base_url .. "/api/tab/llm-proxy"

  -- Prepend instruction prefix to prompt (as the official extension does)
  local full_prompt = M.user_prompt_prefix .. (request.prompt or "")

  -- Debug: log the prompt being sent
  if os.getenv("AMPTAB_DEBUG") then
    local f = io.open("/tmp/amptab_request.txt", "w")
    if f then
      f:write("=== FULL PROMPT ===\n")
      f:write(full_prompt or "nil")
      f:write("\n\n=== CODE_TO_REWRITE ===\n")
      f:write(request.code_to_rewrite or "nil")
      f:write("\n\n=== PROMPT LENGTH ===\n")
      f:write(
        string.format("Chars: %d, Est tokens: %d\n", #full_prompt, math.ceil(#full_prompt / 4))
      )
      f:close()
    end
  end

  local body = vim.json.encode({
    stream = true,
    model = M.defaults.model,
    temperature = M.defaults.temperature,
    max_tokens = request.max_tokens or 1024,
    response_format = { type = "text" },
    prediction = {
      type = "content",
      content = request.code_to_rewrite,
    },
    stop = { "<|editable_region_end|>" },
    prompt = full_prompt,
    user = os.getenv("USER") or "neovim-user",
  })

  local full_text = ""
  local cancelled = false
  local handle = nil

  handle = vim.system({
    "curl",
    "-sS",
    "-X",
    "POST",
    "-H",
    "Content-Type: application/json",
    "-H",
    "Authorization: Bearer " .. api_key,
    "-d",
    body,
    url,
  }, {
    text = true,
    stdout = function(err, data)
      if cancelled then
        return
      end
      if err then
        vim.schedule(function()
          on_error(err)
        end)
        return
      end
      if data then
        local data_str = type(data) == "string" and data or tostring(data)
        for line in data_str:gmatch("[^\r\n]+") do
          local content = parse_sse_chunk(line)
          if content and type(content) == "string" then
            full_text = full_text .. content
            vim.schedule(function()
              on_chunk(content)
            end)
          end
        end
      end
    end,
  }, function(result)
    if cancelled then
      return
    end
    vim.schedule(function()
      -- Debug: log response
      if os.getenv("AMPTAB_DEBUG") then
        local f = io.open("/tmp/amptab_response.txt", "w")
        if f then
          f:write("=== RESPONSE ===\n")
          f:write(full_text)
          f:write("\n\n=== EXIT CODE ===\n")
          f:write(tostring(result.code))
          if result.stderr and result.stderr ~= "" then
            f:write("\n\n=== STDERR ===\n")
            f:write(result.stderr)
          end
          f:close()
        end
      end

      if result.code ~= 0 then
        on_error("Request failed: " .. (result.stderr or "unknown error"))
      else
        on_done(full_text)
      end
    end)
  end)

  return function()
    cancelled = true
    if handle then
      handle:kill("SIGTERM")
    end
  end
end

return M
