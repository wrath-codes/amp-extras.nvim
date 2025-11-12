-- Automated WebSocket test with real client
-- This script starts the server, triggers some events, and a Rust client connects to verify

vim.opt.runtimepath:append(vim.fn.getcwd())

local amp = require("amp_extras")

print("=== Automated WebSocket Integration Test ===")
print()

-- Start server
print("Step 1: Starting WebSocket server...")
local server_result = amp.server_start()
if not server_result then
  print("❌ Failed to start server")
  vim.cmd("cquit!")
end

print(string.format("✅ Server started on port %d", server_result.port))
print(string.format("   Token: %s...", server_result.token:sub(1, 8)))
print()

-- Setup notifications
print("Step 2: Setting up notifications...")
local notif_result = amp.setup_notifications()
if not (notif_result and notif_result.success) then
  print("❌ Failed to setup notifications")
  vim.cmd("cquit!")
end
print("✅ Notifications set up")
print()

-- Write connection info to file for the Rust client
local conn_info = string.format("%d\n%s\n", server_result.port, server_result.token)
local f = io.open("/tmp/amp_ws_test.txt", "w")
if f then
  f:write(conn_info)
  f:close()
  print("✅ Connection info written to /tmp/amp_ws_test.txt")
else
  print("❌ Failed to write connection info")
end
print()

-- Create test file
print("Step 3: Creating test file with content...")
local tmpfile = vim.fn.tempname()
vim.cmd("edit " .. tmpfile)
vim.api.nvim_buf_set_lines(0, 0, -1, false, {
  "Line 1: Testing WebSocket notifications",
  "Line 2: This is a test file",
  "Line 3: With multiple lines",
  "Line 4: For notification testing"
})
print("✅ Test file created")
print()

-- Trigger various events
print("Step 4: Triggering notification events...")
print("   - Moving cursor...")
vim.api.nvim_win_set_cursor(0, {2, 10})
vim.wait(100)

print("   - Visual selection (word)...")
vim.cmd("normal! viw")
vim.wait(100)
vim.cmd("normal! \\<Esc>")

print("   - Visual selection (line)...")
vim.cmd("normal! V")
vim.wait(100)
vim.cmd("normal! \\<Esc>")

print("   - Creating split...")
local tmpfile2 = vim.fn.tempname()
vim.cmd("split " .. tmpfile2)
vim.api.nvim_buf_set_lines(0, 0, -1, false, {"Second file content"})
vim.wait(100)

print("✅ Events triggered")
print()

print("╔════════════════════════════════════════════════════════╗")
print("║  Server is running and ready for client connection!   ║")
print("╚════════════════════════════════════════════════════════╝")
print()
print("In another terminal, run:")
print(string.format("  export WS_PORT=%d", server_result.port))
print(string.format("  export WS_TOKEN=%s", server_result.token))
print("  cargo test --test websocket_client -- --nocapture")
print()
print("Or use the helper script:")
print(string.format("  ./tests/test_websocket.sh %d %s", server_result.port, server_result.token))
print()
print("Then move cursor, make selections, etc. in this Neovim instance")
print()
print("Press Ctrl+C when done testing")
print()

-- Keep Neovim running for manual testing
-- User will Ctrl+C to exit
while true do
  vim.wait(1000)
end
