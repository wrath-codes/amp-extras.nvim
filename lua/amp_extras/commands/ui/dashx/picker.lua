local n = require("nui-components")
local api = require("amp_extras.commands.dashx.api")
local form = require("amp_extras.commands.ui.dashx.form")
local session = require("amp_extras.commands.session")

local M = {}

local current_renderer = nil
local resize_augroup = vim.api.nvim_create_augroup("DashXPickerResize", { clear = true })
local resize_timer = vim.loop.new_timer()

-- Persistent state for resize handling
local _state = {
  all_prompts = {},
  search_query = "",
  nodes = {},
  selected_index = 1,
  selected_ids = {},
}

function M.show(opts)
  opts = opts or {}
  local is_resize = opts.keep_state
  -- If opening fresh, reset state
  if not is_resize then
    _state.all_prompts = {}
    _state.search_query = ""
    _state.nodes = {}
    _state.selected_index = 1
    _state.selected_ids = {}
  end

  if is_resize then
    _state.selected_index = 1
    _state.selected_ids = {}
  end

  -- Close existing renderer if any (e.g. during resize)
  if current_renderer then
    pcall(function()
      current_renderer:close()
    end)
    current_renderer = nil
  end

  if not is_resize then
    vim.api.nvim_create_autocmd("BufEnter", {
      pattern = "*",
      callback = function()
        if current_renderer then
          current_renderer:close()
        end
      end,
    })
  end

  -- Helper to ensure we get a valid background color for our selection cursor
  local function setup_highlights()
    -- Ensure the focus highlight group links to a standard selection group
    vim.api.nvim_set_hl(0, "NuiComponentsSelectNodeFocused", { link = "PmenuSel", default = true })

    local function get_color(group_name, attr)
      local ok, hl = pcall(vim.api.nvim_get_hl, 0, { name = group_name, link = false })
      if not ok then
        return nil
      end
      return hl and hl[attr] or nil
    end

    -- Create a specific icon highlight that combines the Selection BG with DiagnosticWarn FG
    -- This prevents the selection foreground (usually white) from overriding the warning color
    local bg = get_color("PmenuSel", "bg") or get_color("Visual", "bg") or "#2c323c"
    local warn_fg = get_color("DiagnosticWarn", "fg") or get_color("WarningMsg", "fg") or "#e5c07b" -- Orange/Yellow

    vim.api.nvim_set_hl(0, "DashXSelectIcon", { fg = warn_fg, bg = bg, force = true })

    -- Keymap Highlights
    vim.api.nvim_set_hl(0, "DashXKey", { link = "Special", default = true })
    vim.api.nvim_set_hl(0, "DashXDesc", { link = "Comment", default = true })
  end

  setup_highlights()

  -- Calculate dimensions based on current screen size
  local total_width = vim.o.columns
  local total_height = vim.o.lines

  local width = math.min(120, math.floor(total_width * 0.9))
  local height = math.min(40, math.floor(total_height * 0.8))

  -- Determine layout mode
  local is_compact = width < 100

  -- Create a buffer for the preview
  local preview_buf = vim.api.nvim_create_buf(false, true)

  local renderer = n.create_renderer({
    width = width,
    height = height,
  })
  current_renderer = renderer

  -- Reactive signal for the UI list
  local signal = n.create_signal({
    nodes = _state.nodes,
  })

  local update_preview -- Forward declaration

  -- Logic: Filter the cached prompts based on query
  local function filter_list()
    local query = _state.search_query:lower()
    -- Split query into terms for multi-word search
    local query_terms = vim.split(query, "%s+", { trimempty = true })
    local nodes = {}

    for _, p in ipairs(_state.all_prompts) do
      local match = true
      if #query_terms > 0 then
        -- Prepare searchable text: Title + Tags
        local tags_str = (p.tags and table.concat(p.tags, " ") or "")
        local full_text = (p.title .. " " .. tags_str):lower()

        for _, term in ipairs(query_terms) do
          if not full_text:find(term, 1, true) then
            match = false
            break
          end
        end
      end

      if match then
        table.insert(
          nodes,
          n.option(
            p.title .. (p.usage_count > 0 and string.format(" (Used: %d)", p.usage_count) or ""),
            { id = p.id, _prompt = p }
          )
        )
      end
    end
    -- Update signal
    signal.nodes = nodes
    _state.nodes = nodes
    _state.selected_index = 1
    _state.selected_ids = {}

    -- Manual force update
    vim.schedule(function()
      -- Ensure this renderer is still active
      if renderer ~= current_renderer then
        return
      end

      local list = renderer:get_component_by_id("prompt_list")
      if list then
        local tree = list.tree or (list.get_tree and list:get_tree())

        if tree then
          tree:set_nodes(nodes)
          tree:render()

          -- Reset visual cursor to top
          if list.winid and vim.api.nvim_win_is_valid(list.winid) then
            pcall(vim.api.nvim_win_set_cursor, list.winid, { 1, 0 })
          end

          -- Update preview for the new first item
          if nodes[1] then
            update_preview(nodes[1])
          end
        end
      end
    end)
  end

  -- Logic: Fetch from DB and update cache
  local function fetch_data()
    local ok, result = pcall(api.list_prompts)
    if ok then
      _state.all_prompts = result or {}
      -- Normalize tags
      for _, p in ipairs(_state.all_prompts) do
        if p.tags and type(p.tags) == "string" and p.tags ~= "" then
          local ok, decoded = pcall(vim.json.decode, p.tags)
          if ok and type(decoded) == "table" then
            p.tags = decoded
          else
            p.tags = nil
          end
        end
      end
      -- Update the UI
      filter_list()
    else
      vim.notify("Failed to load prompts: " .. tostring(result), vim.log.levels.ERROR)
    end
  end

  -- Initial load (only if not resizing)
  if not is_resize then
    vim.schedule(fetch_data)
  end

  -- ... rest of style definitions ...
  local window_style = {
    highlight = {
      -- Ensure persistent background
      Normal = "NormalFloat",
      NormalNC = "NormalFloat",
      FloatBorder = "DiagnosticError",
      FloatTitle = "DiagnosticError",
    },
  }

  -- Specific style for the list
  local select_window_style = window_style

  -- Helper to choose best component for preview
  local PreviewComponent = n.buffer

  local preview_props = {
    id = "preview_buffer",
    buf = preview_buf,
    autoscroll = false,
    flex = 2,
    border_label = {
      text = "Preview",
      icon = "",
    },
    window = window_style, -- Use the shared style with correct highlights
  }

  -- Logic: Update preview based on selected node
  update_preview = function(node)
    local prompt = node._prompt
    if prompt then
      vim.schedule(function()
        -- Ensure buffer is still valid
        if not vim.api.nvim_buf_is_valid(preview_buf) then
          return
        end

        -- Enable Markdown highlighting
        vim.bo[preview_buf].buftype = "nofile"
        vim.bo[preview_buf].swapfile = false

        if vim.bo[preview_buf].filetype ~= "markdown" then
          vim.bo[preview_buf].filetype = "markdown"
        end

        -- Note: Window-local options (wrap, etc.) are best set when the window is known.
        -- Since we don't strictly know the window ID here without getting the component,
        -- we rely on the component's mounting logic or defaults.
        -- But we can try to find the window displaying this buffer if needed.
        local winids = vim.fn.win_findbuf(preview_buf)
        for _, winid in ipairs(winids) do
          vim.wo[winid].wrap = true
          vim.wo[winid].signcolumn = "no"
          vim.wo[winid].conceallevel = 2
          vim.wo[winid].concealcursor = "n"
          vim.wo[winid].spell = false
          vim.wo[winid].foldenable = false
        end

        -- Ensure TreeSitter is active
        local ok = pcall(vim.treesitter.start, preview_buf, "markdown")
        if not ok then
          vim.bo[preview_buf].syntax = "markdown"
        end

        local lines = {}

        -- Title
        table.insert(lines, "# " .. prompt.title)

        if prompt.description and prompt.description ~= "" and prompt.description ~= vim.NIL then
          table.insert(lines, "")
          table.insert(lines, "**Description:** " .. prompt.description)
        end

        table.insert(lines, "")

        -- Metadata Table
        table.insert(lines, "| Metric | Value |")
        table.insert(lines, "| --- | --- |")
        table.insert(lines, string.format("| **Usage Count** | %d |", prompt.usage_count))

        if prompt.last_used_at then
          local last_used = os.date("%Y-%m-%d %H:%M", prompt.last_used_at)
          table.insert(lines, string.format("| **Last Used** | %s |", last_used))
        else
          table.insert(lines, "| **Last Used** | Never |")
        end

        if prompt.updated_at then
          local updated = os.date("%Y-%m-%d %H:%M", prompt.updated_at)
          table.insert(lines, string.format("| **Updated** | %s |", updated))
        end

        table.insert(lines, "")

        -- Tags (Separate Section)
        if prompt.tags and #prompt.tags > 0 then
          table.insert(lines, "## Tags")
          table.insert(lines, "")

          local tag_items = {}
          for _, t in ipairs(prompt.tags) do
            -- Sanitize tags to ensure no newlines
            local safe_tag = string.gsub(t, "[\r\n]", " ")
            table.insert(tag_items, "`" .. safe_tag .. "`")
          end
          -- Join with spaces for a "chip" list look
          table.insert(lines, table.concat(tag_items, " "))
          table.insert(lines, "")
        end
        table.insert(lines, "---")
        table.insert(lines, "")
        table.insert(lines, "## Content")
        table.insert(lines, "")

        -- Content
        local content_lines = vim.split(prompt.content, "\n")
        for _, line in ipairs(content_lines) do
          table.insert(lines, line)
        end

        -- Write to buffer
        vim.api.nvim_buf_set_lines(preview_buf, 0, -1, false, lines)

        -- Trigger render-markdown if available
        if package.loaded["render-markdown"] then
          vim.schedule(function()
            if vim.api.nvim_buf_is_valid(preview_buf) then
              local ok, rm = pcall(require, "render-markdown")
              if ok and rm.enable then
                vim.api.nvim_buf_call(preview_buf, rm.enable)
              end
            end
          end)
        end
      end)
    end
  end

  -- Shared logic to submit a prompt
  local function submit_prompt(prompt)
    if not prompt then
      return
    end

    -- Usage tracking
    pcall(api.use_prompt, prompt.id)

    -- Send to Amp (Assuming amp global or module)
    local ok, amp_msg = pcall(require, "amp.message")
    if ok then
      amp_msg.send_message(prompt.content)
    else
      vim.notify("Sent prompt to Amp: " .. prompt.title)
    end

    if current_renderer then
      pcall(function()
        current_renderer:close()
      end)
      current_renderer = nil
    end
  end

  local function update_keymaps()
    local selected_count = 0
    for _ in pairs(_state.selected_ids) do
      selected_count = selected_count + 1
    end

    local actions = {}
    if selected_count == 0 then
      table.insert(actions, { key = "<Enter>", desc = "Send" })
    end
    table.insert(actions, { key = "<Tab>", desc = "Select" })
    table.insert(actions, { key = "<S-Tab>", desc = "Deselect" })
    table.insert(actions, { key = "<C-n>", desc = "New" })

    if selected_count > 0 then
      table.insert(actions, { key = "<C-t>", desc = "Edit Tags" })
      table.insert(actions, { key = "<C-d>", desc = "Delete (" .. selected_count .. ")" })
    else
      table.insert(actions, { key = "<C-e>", desc = "Edit" })
      table.insert(actions, { key = "<C-x>", desc = "Execute" })
      table.insert(actions, { key = "<C-c>", desc = "Copy" })
      table.insert(actions, { key = "<C-S-s>", desc = "Session" })
      table.insert(actions, { key = "<C-d>", desc = "Delete" })
    end

    table.insert(actions, { key = "q/<CR>", desc = "Close" })

    local comp = renderer:get_component_by_id("keymaps_display")
    if comp and comp.bufnr and vim.api.nvim_buf_is_valid(comp.bufnr) then
      -- Manual Centering & Multiline Logic
      local win_width = 0
      if comp.winid and vim.api.nvim_win_is_valid(comp.winid) then
        win_width = vim.api.nvim_win_get_width(comp.winid)
      else
        win_width = renderer._layout_options and renderer._layout_options.width or 80
      end

      local final_lines = {}
      local all_highlights = {} -- Flat list of {line_idx, group, start_col, end_col}

      -- Helper to process a chunk of actions
      local function process_chunk(chunk, line_idx)
        local line_text = ""
        local chunk_highlights = {}

        for i, action in ipairs(chunk) do
          if i > 1 then
            line_text = line_text .. "   " -- 3 spaces separator
          end

          -- Key
          local key_start = #line_text
          line_text = line_text .. action.key
          table.insert(chunk_highlights, { "DashXKey", key_start, #line_text })

          -- Space
          line_text = line_text .. " "

          -- Desc
          local desc_start = #line_text
          line_text = line_text .. action.desc
          table.insert(chunk_highlights, { "DashXDesc", desc_start, #line_text })
        end

        local text_width = vim.fn.strdisplaywidth(line_text)
        local padding = math.max(0, math.floor((win_width - text_width) / 2))
        local centered_text = string.rep(" ", padding) .. line_text

        table.insert(final_lines, centered_text)

        -- Adjust highlights for padding and add to main list
        for _, hl in ipairs(chunk_highlights) do
          table.insert(all_highlights, { line_idx, hl[1], padding + hl[2], padding + hl[3] })
        end
      end

      -- Split actions into chunks of 4
      local current_chunk = {}
      local current_line_idx = 0

      for i, action in ipairs(actions) do
        table.insert(current_chunk, action)
        if #current_chunk == 4 then
          process_chunk(current_chunk, current_line_idx)
          current_chunk = {}
          current_line_idx = current_line_idx + 1
        end
      end

      -- Process remaining items
      if #current_chunk > 0 then
        process_chunk(current_chunk, current_line_idx)
      end

      vim.bo[comp.bufnr].modifiable = true
      -- Force enough lines in the buffer to avoid "Index out of bounds" or weird scrolling if component height is fixed
      vim.api.nvim_buf_set_lines(comp.bufnr, 0, -1, false, final_lines)
      vim.bo[comp.bufnr].modifiable = false

      -- Force refresh window view to ensure lines are visible
      if comp.winid and vim.api.nvim_win_is_valid(comp.winid) then
        -- Reset view to top
        vim.api.nvim_win_set_cursor(comp.winid, { 1, 0 })
      end

      -- Apply highlights
      local ns_id = vim.api.nvim_create_namespace("dashx_keymaps")
      vim.api.nvim_buf_clear_namespace(comp.bufnr, ns_id, 0, -1)

      for _, hl in ipairs(all_highlights) do
        -- hl = {line_idx, group, start, end}
        vim.api.nvim_buf_add_highlight(comp.bufnr, ns_id, hl[2], hl[1], hl[3], hl[4])
      end
    end
  end

  -- Forward declaration for helper
  local move_selection_helper

  local function toggle_selection()
    local node = _state.nodes[_state.selected_index]
    if not node or not node._prompt then
      return
    end

    local id = node._prompt.id
    if _state.selected_ids[id] then
      _state.selected_ids[id] = nil
    else
      _state.selected_ids[id] = true
      -- Move to next item only on select
      move_selection_helper(1)
    end

    update_keymaps()

    local list = renderer:get_component_by_id("prompt_list")
    if list then
      local tree = list.tree or (list.get_tree and list:get_tree())
      if tree then
        tree:render()
      end
    end
  end

  local function toggle_selection_back()
    local node = _state.nodes[_state.selected_index]
    if not node or not node._prompt then
      return
    end

    local id = node._prompt.id
    if _state.selected_ids[id] then
      _state.selected_ids[id] = nil
    end

    -- Move back after unselecting (optional, but usually intuitive for "Shift-Tab")
    move_selection_helper(-1)

    update_keymaps()

    local list = renderer:get_component_by_id("prompt_list")
    if list then
      local tree = list.tree or (list.get_tree and list:get_tree())
      if tree then
        tree:render()
      end
    end
  end

  local function submit_selection()
    -- If multiple selection is active, disable send
    local selected_count = 0
    for _ in pairs(_state.selected_ids) do
      selected_count = selected_count + 1
    end

    if selected_count > 0 then
      return
    end

    local node = _state.nodes[_state.selected_index]
    if node and node._prompt then
      submit_prompt(node._prompt)
    end
  end

  -- Shared mappings for CRUD operations
  local function get_crud_mappings(component_id)
    local function move_selection(direction)
      local count = #_state.nodes
      if count == 0 then
        return
      end

      local new_index = _state.selected_index + direction
      if new_index < 1 then
        new_index = 1
      end
      if new_index > count then
        new_index = count
      end

      _state.selected_index = new_index

      -- Update visual selection
      local list = renderer:get_component_by_id("prompt_list")
      if list then
        local winid = list.winid
        if winid and vim.api.nvim_win_is_valid(winid) then
          pcall(vim.api.nvim_win_set_cursor, winid, { new_index, 0 })

          -- Force re-render for highlight update (optional if nui handles it, but safer)
          local tree = list.tree or (list.get_tree and list:get_tree())
          if tree then
            tree:render()
          end
        end
      end

      -- Update preview
      if _state.nodes[new_index] then
        update_preview(_state.nodes[new_index])
      end
    end

    -- Assign to the outer variable so it's accessible by toggle_selection
    move_selection_helper = move_selection

    return {
      {
        mode = { "n", "i" },
        key = "<CR>",
        handler = submit_selection,
      },
      {
        mode = { "n", "i" },
        key = "<Tab>",
        handler = toggle_selection,
      },
      {
        mode = { "n", "i" },
        key = "<S-Tab>",
        handler = toggle_selection_back,
      },
      {
        mode = { "n" },
        key = "j",
        handler = function()
          move_selection(1)
        end,
      },
      {
        mode = { "n" },
        key = "k",
        handler = function()
          move_selection(-1)
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-j>",
        handler = function()
          move_selection(1)
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-k>",
        handler = function()
          move_selection(-1)
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-x>",
        handler = function()
          local selected_count = 0
          for _ in pairs(_state.selected_ids) do
            selected_count = selected_count + 1
          end
          if selected_count > 0 then
            return
          end

          local node = _state.nodes[_state.selected_index]
          if node and node._prompt then
            renderer:close()
            pcall(api.use_prompt, node._prompt.id)
            session._run_execute(node._prompt.content)
          end
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-c>",
        handler = function()
          local selected_count = 0
          for _ in pairs(_state.selected_ids) do
            selected_count = selected_count + 1
          end
          if selected_count > 0 then
            return
          end

          local node = _state.nodes[_state.selected_index]
          if node and node._prompt then
            vim.fn.setreg("+", node._prompt.content)
            vim.notify("Copied prompt to clipboard", vim.log.levels.INFO)
            renderer:close()
          end
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-S-s>",
        handler = function()
          local selected_count = 0
          for _ in pairs(_state.selected_ids) do
            selected_count = selected_count + 1
          end
          if selected_count > 0 then
            return
          end

          local node = _state.nodes[_state.selected_index]
          if node and node._prompt then
            renderer:close()
            pcall(api.use_prompt, node._prompt.id)
            session._run_start_with_message(node._prompt.content)
          end
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-n>",
        handler = function()
          renderer:close()
          form.show({
            mode = "create",
            on_success = function()
              M.show()
            end,
          })
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-e>",
        handler = function()
          -- If in bulk mode, C-e is disabled
          local selected_count = 0
          for _ in pairs(_state.selected_ids) do
            selected_count = selected_count + 1
          end
          if selected_count > 0 then
            return
          end

          -- Always try to get the selected node from the list component
          local node = _state.nodes[_state.selected_index]
          if node and node._prompt then
            renderer:close()
            form.show({
              mode = "edit",
              prompt = node._prompt,
              on_success = function()
                M.show()
              end,
            })
          end
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-t>",
        handler = function()
          local targets = {}
          local selected_count = 0
          for _ in pairs(_state.selected_ids) do
            selected_count = selected_count + 1
          end

          if selected_count > 0 then
            for _, p in ipairs(_state.all_prompts) do
              if _state.selected_ids[p.id] then
                table.insert(targets, p)
              end
            end
          else
            local node = _state.nodes[_state.selected_index]
            if node and node._prompt then
              table.insert(targets, node._prompt)
            end
          end

          if #targets == 0 then
            return
          end

          renderer:close()
          form.show({
            mode = "bulk_tags",
            prompt = (#targets == 1) and targets[1] or nil,
            on_success = function(new_tags)
              for _, p in ipairs(targets) do
                pcall(api.update_prompt, p.id, p.title, p.content, new_tags)
              end
              M.show()
            end,
          })
        end,
      },
      {
        mode = { "n", "i" },
        key = "<C-d>",
        handler = function()
          local selected_count = 0
          for _ in pairs(_state.selected_ids) do
            selected_count = selected_count + 1
          end

          if selected_count > 0 then
            local choice =
              vim.fn.confirm("Delete " .. selected_count .. " prompts?", "&Yes\n&No", 2)
            if choice == 1 then
              for id, _ in pairs(_state.selected_ids) do
                pcall(api.delete_prompt, id)
              end
              _state.selected_ids = {}
              update_keymaps()
              fetch_data()
            end
            return
          end

          local node = _state.nodes[_state.selected_index]
          if node and node._prompt then
            local choice =
              vim.fn.confirm("Delete prompt '" .. node._prompt.title .. "'?", "&Yes\n&No", 2)
            if choice == 1 then
              pcall(api.delete_prompt, node._prompt.id)
              fetch_data()
            end
          end
        end,
      },
      {
        mode = { "n" },
        key = "q",
        handler = function()
          if renderer then
            renderer:close()
          end
        end,
      },
    }
  end

  local body = function()
    -- Define components
    local search_component = n.text_input({
      id = "search_input",
      placeholder = "Search prompts by title or tag names...",
      autofocus = true,
      value = _state.search_query, -- Restore search query
      size = 3,
      max_lines = 1,
      border_label = {
        text = "Search",
        icon = "",
      },
      window = window_style,
      on_change = function(value)
        _state.search_query = value
        filter_list()
      end,
      on_mount = function(component)
        -- Force map <CR> in Insert mode to submit, overriding nui defaults
        if component.bufnr then
          vim.keymap.set("i", "<CR>", submit_selection, { buffer = component.bufnr, nowait = true })
        end
      end,
      mappings = get_crud_mappings,
    })

    local list_component = n.select({
      id = "prompt_list",
      data = signal.nodes,
      multiselect = false,
      flex = 1,
      is_focusable = false,
      border_label = {
        text = "Prompts",
        icon = "",
      },
      window = select_window_style,
      -- Enable cursorline for the unified background effect
      on_mount = function(component)
        if component.winid and vim.api.nvim_win_is_valid(component.winid) then
          vim.wo[component.winid].cursorline = true
          vim.wo[component.winid].cursorlineopt = "line"
        end
      end,
      -- Custom rendering to show usage stats
      prepare_node = function(is_selected, node, component)
        local prompt = node._prompt
        local line = n.line()

        local base_hl = "NuiComponentsSelectOption"
        local focused_hl = "NuiComponentsSelectNodeFocused"

        -- Determine focus via tree cursor
        local focused = false
        if component and component.tree and component.tree.get_node then
          local focused_node = component.tree:get_node()
          focused = focused_node and focused_node.id == node.id
        end

        local hl = focused and focused_hl or base_hl

        if not prompt then
          local text = node.text or ""
          line:append(n.text(focused and "> " or "  ", hl))
          line:append(n.text(text, hl))
          return line
        end

        local is_checked = _state.selected_ids[prompt.id]
        local check_text = is_checked and " " or " "
        local check_hl = is_checked and "String" or "Comment"

        if focused then
          -- Focused line
          line:append(n.text("> ", "DashXSelectIcon"))
          line:append(n.text(check_text, check_hl))
          line:append(n.text(prompt.title, focused_hl))

          if prompt.usage_count > 0 then
            line:append(n.text(" ", focused_hl))
            line:append(n.text(string.format("(Used: %d)", prompt.usage_count), focused_hl))
          end
        else
          -- Unfocused line
          line:append(n.text("  ", base_hl))
          line:append(n.text(check_text, check_hl))
          line:append(n.text(prompt.title, "Function"))

          if prompt.usage_count > 0 then
            line:append(n.text(" ", base_hl))
            line:append(n.text(string.format("(Used: %d)", prompt.usage_count), "Comment"))
          end
        end

        return line
      end,
      mappings = get_crud_mappings,
      on_select = function(node, component)
        local prompt = node._prompt
        if prompt then
          submit_prompt(prompt)
        end
      end,
      on_change = function(node)
        update_preview(node)
        -- Force redraw for selection highlight update if needed
        vim.cmd("redraw")
      end,
    })

    local preview_component = PreviewComponent(preview_props)

    local keymaps_component = n.paragraph({
      id = "keymaps_display",
      lines = " \n ", -- Initialize with 2 lines to ensure height
      align = "center",
      is_focusable = false,
      size = 5,
      border_label = {
        text = "Keymaps",
        icon = "",
      },
      window = window_style,
      on_mount = function()
        vim.schedule(update_keymaps)
      end,
    })

    if is_compact then
      -- Stacked Layout
      return n.rows(
        search_component,
        n.gap(1),
        list_component,
        n.gap(1),
        preview_component,
        n.gap(1),
        keymaps_component
      )
    else
      -- Split Layout
      return n.rows(
        { flex = 1 },
        n.columns(
          { flex = 1 },
          -- Left Pane: Search & List (30%)
          n.rows({ flex = 1 }, search_component, list_component),
          -- Right Pane: Preview (70%)
          preview_component
        ),
        n.gap(1),
        keymaps_component
      )
    end
  end

  renderer:render(body)

  -- Register Autocmd for resize
  vim.api.nvim_create_autocmd("VimResized", {
    group = resize_augroup,
    callback = function()
      resize_timer:stop()
      resize_timer:start(
        50,
        0,
        vim.schedule_wrap(function()
          M.show({ keep_state = true })
        end)
      )
    end,
  })

  -- Cleanup autocmd when renderer closes
  local original_close = renderer.close
  renderer.close = function(self)
    resize_timer:stop()
    vim.api.nvim_clear_autocmds({ group = resize_augroup })
    if vim.api.nvim_buf_is_valid(preview_buf) then
      vim.api.nvim_buf_delete(preview_buf, { force = true })
    end
    if original_close then
      original_close(self)
    end
  end
end

return M
