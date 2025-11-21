-- Setup basic environment
vim.cmd([[set runtimepath=/Users/wrath/projects/amp-extras.nvim]])
vim.cmd([[set runtimepath+=/Users/wrath/projects/amp-extras.nvim/../nui-components.nvim]]) -- Assuming nui-components is available
vim.cmd([[packadd nvim-oxi]]) -- Assuming nvim-oxi is packed or available

-- Mock require("amp_extras.ffi") if strictly needed, but we rely on the real one loading
-- However, the binary .so needs to be built and in place.

-- Load plugin
require("amp_extras").setup({})

-- Check commands
local commands = vim.api.nvim_get_commands({})
local dashx = commands["AmpDashX"]

if dashx then
    print("SUCCESS: AmpDashX command found")
else
    print("FAILURE: AmpDashX command NOT found")
    print("Available commands:")
    for k, v in pairs(commands) do
        if k:match("^Amp") then
            print("- " .. k)
        end
    end
    os.exit(1)
end
