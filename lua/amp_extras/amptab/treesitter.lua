--- AmpTab treesitter-aware context building
--- Uses AST node boundaries for smarter FIM region selection (SAFIM approach)
--- @module 'amp_extras.amptab.treesitter'
local M = {}

---Node types that define meaningful code blocks for different languages
---@type table<string, string[]>
M.block_node_types = {
  -- Common across languages
  default = {
    "function_definition",
    "function_declaration",
    "method_definition",
    "method_declaration",
    "class_definition",
    "class_declaration",
    "if_statement",
    "for_statement",
    "while_statement",
    "try_statement",
    "with_statement",
    "match_statement",
    "block",
  },
  lua = {
    "function_declaration",
    "function_definition",
    "local_function",
    "if_statement",
    "for_statement",
    "while_statement",
    "repeat_statement",
    "do_statement",
    "return_statement",
    "chunk",
  },
  python = {
    "function_definition",
    "class_definition",
    "if_statement",
    "for_statement",
    "while_statement",
    "try_statement",
    "with_statement",
    "match_statement",
    "decorated_definition",
    "async_function_definition",
  },
  rust = {
    "function_item",
    "impl_item",
    "struct_item",
    "enum_item",
    "trait_item",
    "if_expression",
    "for_expression",
    "while_expression",
    "loop_expression",
    "match_expression",
    "block",
  },
  typescript = {
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "class_declaration",
    "if_statement",
    "for_statement",
    "while_statement",
    "try_statement",
    "switch_statement",
  },
  javascript = {
    "function_declaration",
    "function_expression",
    "arrow_function",
    "method_definition",
    "class_declaration",
    "if_statement",
    "for_statement",
    "while_statement",
    "try_statement",
    "switch_statement",
  },
  go = {
    "function_declaration",
    "method_declaration",
    "type_declaration",
    "if_statement",
    "for_statement",
    "switch_statement",
    "select_statement",
    "block",
  },
}

---Check if treesitter is available for buffer
---@param bufnr number
---@return boolean
function M.is_available(bufnr)
  local ok, parser = pcall(vim.treesitter.get_parser, bufnr)
  return ok and parser ~= nil
end

---Get block node types for a filetype
---@param ft string
---@return string[]
function M.get_block_types(ft)
  return M.block_node_types[ft] or M.block_node_types.default
end

---Check if a node type is a block type
---@param node_type string
---@param ft string
---@return boolean
local function is_block_type(node_type, ft)
  local block_types = M.get_block_types(ft)
  for _, t in ipairs(block_types) do
    if node_type == t then
      return true
    end
  end
  return false
end

---Find the smallest enclosing block node containing the cursor
---@param bufnr number
---@param row number 0-indexed
---@param col number 0-indexed
---@return TSNode|nil node, string|nil node_type
function M.find_enclosing_block(bufnr, row, col)
  local ok, parser = pcall(vim.treesitter.get_parser, bufnr)
  if not ok or not parser then
    return nil, nil
  end

  local tree = parser:parse()[1]
  if not tree then
    return nil, nil
  end

  local root = tree:root()
  local ft = vim.bo[bufnr].filetype

  local node = root:named_descendant_for_range(row, col, row, col)
  if not node then
    return nil, nil
  end

  -- Walk up the tree to find a block node
  while node do
    local node_type = node:type()
    if is_block_type(node_type, ft) then
      return node, node_type
    end
    node = node:parent()
  end

  return nil, nil
end

---Find the function/method containing the cursor
---@param bufnr number
---@param row number 0-indexed
---@param col number 0-indexed
---@return TSNode|nil node, string|nil node_type
function M.find_enclosing_function(bufnr, row, col)
  local ok, parser = pcall(vim.treesitter.get_parser, bufnr)
  if not ok or not parser then
    return nil, nil
  end

  local tree = parser:parse()[1]
  if not tree then
    return nil, nil
  end

  local root = tree:root()
  local ft = vim.bo[bufnr].filetype

  local function_types = {
    "function_definition",
    "function_declaration",
    "method_definition",
    "method_declaration",
    "local_function",
    "function_item",
    "arrow_function",
    "function_expression",
    "async_function_definition",
  }

  local node = root:named_descendant_for_range(row, col, row, col)
  if not node then
    return nil, nil
  end

  while node do
    local node_type = node:type()
    for _, t in ipairs(function_types) do
      if node_type == t then
        return node, node_type
      end
    end
    node = node:parent()
  end

  return nil, nil
end

---Find the class/struct/impl containing the cursor
---@param bufnr number
---@param row number 0-indexed
---@param col number 0-indexed
---@return TSNode|nil node, string|nil node_type
function M.find_enclosing_class(bufnr, row, col)
  local ok, parser = pcall(vim.treesitter.get_parser, bufnr)
  if not ok or not parser then
    return nil, nil
  end

  local tree = parser:parse()[1]
  if not tree then
    return nil, nil
  end

  local root = tree:root()

  local class_types = {
    "class_definition",
    "class_declaration",
    "struct_item",
    "impl_item",
    "trait_item",
    "enum_item",
    "type_declaration",
  }

  local node = root:named_descendant_for_range(row, col, row, col)
  if not node then
    return nil, nil
  end

  while node do
    local node_type = node:type()
    for _, t in ipairs(class_types) do
      if node_type == t then
        return node, node_type
      end
    end
    node = node:parent()
  end

  return nil, nil
end

---@class TSContextRegion
---@field start_row number 0-indexed
---@field start_col number 0-indexed
---@field end_row number 0-indexed
---@field end_col number 0-indexed
---@field node_type string|nil
---@field strategy string "function"|"block"|"fallback"|"class"
---@field class_context string|nil Class header/init for attribute context

---Find __init__ or constructor method within a class node
---@param class_node TSNode
---@param bufnr number
---@return TSNode|nil
local function find_init_method(class_node, bufnr)
  local ft = vim.bo[bufnr].filetype
  local init_names = {
    python = { "__init__" },
    lua = { "new", "init", "_init" },
    rust = { "new", "default" },
    typescript = { "constructor" },
    javascript = { "constructor" },
  }
  local names = init_names[ft] or { "new", "init", "constructor" }

  for child in class_node:iter_children() do
    local child_type = child:type()
    if
      child_type == "function_definition"
      or child_type == "method_definition"
      or child_type == "function_item"
    then
      local name_node = child:field("name")[1]
      if name_node then
        local name = vim.treesitter.get_node_text(name_node, bufnr)
        for _, init_name in ipairs(names) do
          if name == init_name then
            return child
          end
        end
      end
    end
  end
  return nil
end

---Get class header and __init__ as context string
---@param bufnr number
---@param row number 0-indexed
---@param col number 0-indexed
---@param max_lines? number
---@return string|nil class_context
function M.get_class_context(bufnr, row, col, max_lines)
  max_lines = max_lines or 30
  local class_node = M.find_enclosing_class(bufnr, row, col)
  if not class_node then
    return nil
  end

  local class_start, _, _, _ = class_node:range()
  local lines = {}

  -- Get class header (first line with class definition)
  local header = vim.api.nvim_buf_get_lines(bufnr, class_start, class_start + 1, false)[1]
  if header then
    table.insert(lines, header)
  end

  -- Find and include __init__ method
  local init_node = find_init_method(class_node, bufnr)
  if init_node then
    local init_start, _, init_end, _ = init_node:range()
    local init_lines = vim.api.nvim_buf_get_lines(bufnr, init_start, init_end + 1, false)
    local to_take = math.min(#init_lines, max_lines - 1)
    for i = 1, to_take do
      table.insert(lines, init_lines[i])
    end
  end

  if #lines > 1 then
    return table.concat(lines, "\n")
  end
  return nil
end

---Determine optimal editable region using treesitter
---@param bufnr number
---@param cursor_row number 0-indexed
---@param cursor_col number 0-indexed
---@param opts? {max_lines?: number, prefer_function?: boolean}
---@return TSContextRegion
function M.get_editable_region(bufnr, cursor_row, cursor_col, opts)
  opts = opts or {}
  local max_lines = opts.max_lines or 100
  local prefer_function = opts.prefer_function ~= false

  local total_lines = vim.api.nvim_buf_line_count(bufnr)

  -- Get class context for attribute awareness (when inside a method)
  local class_context = M.get_class_context(bufnr, cursor_row, cursor_col, 30)

  -- Try function first if preferred
  if prefer_function then
    local func_node, func_type = M.find_enclosing_function(bufnr, cursor_row, cursor_col)
    if func_node then
      local start_row, start_col, end_row, end_col = func_node:range()
      local line_span = end_row - start_row + 1

      -- If function is reasonably sized, use it
      if line_span <= max_lines then
        return {
          start_row = start_row,
          start_col = start_col,
          end_row = end_row,
          end_col = end_col,
          node_type = func_type,
          strategy = "function",
          class_context = class_context,
        }
      end

      -- Function too large - find inner block
      local block_node, block_type = M.find_enclosing_block(bufnr, cursor_row, cursor_col)
      if block_node and block_node ~= func_node then
        local bs, bc, be, ec = block_node:range()
        if be - bs + 1 <= max_lines then
          return {
            start_row = bs,
            start_col = bc,
            end_row = be,
            end_col = ec,
            node_type = block_type,
            strategy = "block",
            class_context = class_context,
          }
        end
      end
    end
  end

  -- Try any block node
  local block_node, block_type = M.find_enclosing_block(bufnr, cursor_row, cursor_col)
  if block_node then
    local start_row, start_col, end_row, end_col = block_node:range()
    local line_span = end_row - start_row + 1

    if line_span <= max_lines then
      return {
        start_row = start_row,
        start_col = start_col,
        end_row = end_row,
        end_col = end_col,
        node_type = block_type,
        strategy = "block",
        class_context = class_context,
      }
    end
  end

  -- If in a class but block too large, use class strategy with nearby methods
  local class_node, class_type = M.find_enclosing_class(bufnr, cursor_row, cursor_col)
  if class_node then
    local class_start, _, class_end, _ = class_node:range()
    -- Use a window around the cursor within the class
    local half = math.floor(max_lines / 2)
    local region_start = math.max(class_start, cursor_row - half)
    local region_end = math.min(class_end, cursor_row + half)

    return {
      start_row = region_start,
      start_col = 0,
      end_row = region_end,
      end_col = 0,
      node_type = class_type,
      strategy = "class",
      class_context = class_context,
    }
  end

  -- Fallback: use fixed line counts centered on cursor
  local half_lines = math.floor(max_lines / 2)
  local fallback_start = math.max(0, cursor_row - half_lines)
  local fallback_end = math.min(total_lines - 1, cursor_row + half_lines)

  return {
    start_row = fallback_start,
    start_col = 0,
    end_row = fallback_end,
    end_col = 0, -- Will be adjusted by caller
    node_type = nil,
    strategy = "fallback",
    class_context = class_context,
  }
end

---Get sibling context (preceding and following sibling nodes)
---Useful for providing context about related code
---@param bufnr number
---@param cursor_row number 0-indexed
---@param cursor_col number 0-indexed
---@param max_siblings? number
---@return {prev: TSNode[], next: TSNode[]}
function M.get_sibling_context(bufnr, cursor_row, cursor_col, max_siblings)
  max_siblings = max_siblings or 2

  local ok, parser = pcall(vim.treesitter.get_parser, bufnr)
  if not ok or not parser then
    return { prev = {}, next = {} }
  end

  local tree = parser:parse()[1]
  if not tree then
    return { prev = {}, next = {} }
  end

  local root = tree:root()
  local ft = vim.bo[bufnr].filetype

  -- Find the current block node
  local node = root:named_descendant_for_range(cursor_row, cursor_col, cursor_row, cursor_col)
  if not node then
    return { prev = {}, next = {} }
  end

  -- Walk up to find a block node
  while node and not is_block_type(node:type(), ft) do
    node = node:parent()
  end

  if not node then
    return { prev = {}, next = {} }
  end

  local prev_siblings = {}
  local next_siblings = {}

  -- Get previous siblings
  local prev = node:prev_named_sibling()
  local count = 0
  while prev and count < max_siblings do
    table.insert(prev_siblings, 1, prev)
    prev = prev:prev_named_sibling()
    count = count + 1
  end

  -- Get next siblings
  local next_node = node:next_named_sibling()
  count = 0
  while next_node and count < max_siblings do
    table.insert(next_siblings, next_node)
    next_node = next_node:next_named_sibling()
    count = count + 1
  end

  return { prev = prev_siblings, next = next_siblings }
end

---Extract text for a node
---@param node TSNode
---@param bufnr number
---@return string
function M.get_node_text(node, bufnr)
  local start_row, start_col, end_row, end_col = node:range()
  local lines = vim.api.nvim_buf_get_lines(bufnr, start_row, end_row + 1, false)

  if #lines == 0 then
    return ""
  end

  if #lines == 1 then
    return lines[1]:sub(start_col + 1, end_col)
  end

  lines[1] = lines[1]:sub(start_col + 1)
  lines[#lines] = lines[#lines]:sub(1, end_col)

  return table.concat(lines, "\n")
end

---Get signature/header of a function node (first line typically)
---@param node TSNode
---@param bufnr number
---@return string
function M.get_function_signature(node, bufnr)
  local start_row, start_col = node:range()
  local line = vim.api.nvim_buf_get_lines(bufnr, start_row, start_row + 1, false)[1]
  if line then
    return line:sub(start_col + 1)
  end
  return ""
end

return M
