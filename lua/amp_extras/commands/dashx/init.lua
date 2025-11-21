local M = {}
local api = require("amp_extras.commands.dashx.api")

---Seed default prompts if the library is empty
local function seed_defaults()
    local ok, prompts = pcall(api.list_prompts)
    if ok and #prompts == 0 then
        local defaults = {
            {
                title = "Explain Code",
                description = "Detailed explanation of logic and edge cases",
                content = "Explain the following code in detail, focusing on the logic and potential edge cases:",
                tags = {"coding", "explanation"}
            },
            {
                title = "Refactor Selection",
                description = "Improve idiom, performance, and readability",
                content = "Refactor the selected code to be more idiomatic, performant, and readable. Explain your changes.",
                tags = {"coding", "refactor"}
            },
            {
                title = "Find Bugs",
                description = "Analyze for bugs, race conditions, and vulnerabilities",
                content = "Analyze the selected code for potential bugs, race conditions, or security vulnerabilities.",
                tags = {"coding", "debug"}
            },
            {
                title = "Generate Unit Tests",
                description = "Create tests for happy paths and error cases",
                content = "Write comprehensive unit tests for the selected code, covering happy paths and error cases.",
                tags = {"coding", "testing"}
            },
            {
                title = "Add Documentation",
                description = "Add standard documentation comments",
                content = "Add detailed documentation comments to the selected code, following standard conventions.",
                tags = {"coding", "docs"}
            }
        }

        for _, p in ipairs(defaults) do
            pcall(api.create_prompt, p.title, p.description, p.content, p.tags)
        end
        vim.notify("DashX: Seeded default prompts", vim.log.levels.INFO)
    end
end

function M.setup()
    -- Register commands
    vim.api.nvim_create_user_command("AmpDashX", function()
        require("amp_extras.commands.ui.dashx.picker").show()
    end, { desc = "Open DashX Prompt Library" })
    
    -- Auto-seed defaults
    vim.schedule(seed_defaults)
end

return M