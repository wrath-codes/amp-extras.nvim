-- Helper script to start amp-extras WebSocket server in Neovim
-- Source this in Neovim: :luafile tests/start_amp_server.lua

-- Add plugin to runtimepath
vim.opt.runtimepath:append(vim.fn.getcwd())

-- Load the plugin
local amp = require("amp_extras")

print("╔══════════════════════════════════════════════════════════╗")
print("║   Starting amp-extras WebSocket Server                  ║")
print("╚══════════════════════════════════════════════════════════╝")
print("")

-- Start the server
local result, err = amp.server_start()

if err then
  print("❌ Failed to start server: " .. err)
  return
end

print("✅ WebSocket server started successfully!")
print("")
print("Connection Information:")
print("─────────────────────────────────────────────────────────")
print(string.format("  Port:     %d", result.port))
print(string.format("  Token:    %s", result.token))
print(string.format("  Lockfile: %s", result.lockfile))
print("─────────────────────────────────────────────────────────")
print("")

-- Setup notifications
print("Setting up notification autocommands...")
local notif_result, notif_err = amp.setup_notifications()

if notif_err then
  print("⚠️  Failed to setup notifications: " .. notif_err)
else
  print("✅ Notifications enabled!")
  print("   - Cursor movements will send selectionDidChange")
  print("   - Visual selections will send selected text")
  print("   - File changes will send visibleFilesDidChange")
end
print("")

-- Create a test file with some content
print("Creating test file with sample content...")
local test_file = vim.fn.getcwd() .. "/tests/test_file.txt"
vim.cmd("edit " .. test_file)
vim.api.nvim_buf_set_lines(0, 0, -1, false, {
  "Line 1: Welcome to amp-extras WebSocket server test!",
  "Line 2: This is a test file for Amp CLI integration.",
  "Line 3: Try moving your cursor around.",
  "Line 4: Enter visual mode (v) and select some text.",
  "Line 5: The selections will be sent to Amp CLI!",
  "",
  "You can also:",
  "  - Open new files (:e newfile.txt)",
  "  - Create splits (:split, :vsplit)",
  "  - Amp should receive visibleFilesDidChange notifications",
})
print("✅ Test file created: " .. test_file)
print("")

print("╔══════════════════════════════════════════════════════════╗")
print("║   Ready for Amp CLI Connection!                         ║")
print("╚══════════════════════════════════════════════════════════╝")
print("")
print("In another terminal, run:")
print("")
print("  amp --ide \"ws://127.0.0.1:" .. result.port .. "/?auth=" .. result.token .. "\"")
print("")
print("Or if Amp supports environment variables:")
print("")
print("  export AMP_IDE_PORT=" .. result.port)
print("  export AMP_IDE_TOKEN=" .. result.token)
print("  amp --ide ws://127.0.0.1:${AMP_IDE_PORT}/?auth=${AMP_IDE_TOKEN}")
print("")
print("Once connected:")
print("  - Move your cursor in this buffer")
print("  - Enter visual mode (v, V, Ctrl-V) and select text")
print("  - Amp should receive real-time notifications!")
print("")
print("To stop the server: :lua require('amp_extras').server_stop()")
print("")
