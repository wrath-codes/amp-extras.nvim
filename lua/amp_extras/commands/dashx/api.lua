local ffi = require("amp_extras.ffi")

local M = {}

---@class Prompt
---@field id string
---@field title string
---@field description string?
---@field content string
---@field tags string[]?
---@field usage_count number
---@field last_used_at number?
---@field created_at number
---@field updated_at number

---List all prompts
---@return Prompt[]
function M.list_prompts()
    local result = ffi.call("prompts.list", {})
    if result.error then
        error(result.message)
    end
    return result.prompts
end

---Create a new prompt
---@param title string
---@param description string?
---@param content string
---@param tags string[]?
---@return Prompt
function M.create_prompt(title, description, content, tags)
    local result = ffi.call("prompts.create", {
        title = title,
        description = description,
        content = content,
        tags = tags
    })
    if result.error then
        error(result.message)
    end
    return result
end

---Update an existing prompt
---@param id string
---@param title string
---@param description string?
---@param content string
---@param tags string[]?
function M.update_prompt(id, title, description, content, tags)
    local result = ffi.call("prompts.update", {
        id = id,
        title = title,
        description = description,
        content = content,
        tags = tags
    })
    if result.error then
        error(result.message)
    end
    return true
end

---Delete a prompt
---@param id string
function M.delete_prompt(id)
    local result = ffi.call("prompts.delete", { id = id })
    if result.error then
        error(result.message)
    end
    return true
end

---Record usage of a prompt
---@param id string
function M.use_prompt(id)
    local result = ffi.call("prompts.use", { id = id })
    if result.error then
        error(result.message)
    end
    return true
end

return M
