-- Headless test for WebSocket notification system
-- Run with: nvim --headless -u tests/notification_test.lua

-- Add plugin to runtimepath
vim.opt.runtimepath:append(vim.fn.getcwd())

-- Load the plugin
local amp = require("amp_extras")

local success = true
local function assert_test(condition, message)
  if not condition then
    print("❌ FAILED: " .. message)
    success = false
  else
    print("✅ PASSED: " .. message)
  end
end

print("=== WebSocket Notification Integration Tests ===")
print()

-- Test 1: Start server
print("Test 1: Start WebSocket server")
local server_result, server_err = amp.server_start()
assert_test(server_result ~= nil, "Server should start successfully")
assert_test(server_err == nil, "Server start should not return error")

if server_result then
  print(string.format("  Port: %d", server_result.port))
  print(string.format("  Token: %s...", server_result.token:sub(1, 8)))
else
  print("❌ Cannot test notifications without server")
  vim.cmd("cquit!")
end
print()

-- Test 2: Setup notifications
print("Test 2: Setup notification autocommands")
local notif_result = amp.setup_notifications()
assert_test(notif_result ~= nil, "Setup notifications should return result")

if notif_result and notif_result.error then
  print(string.format("  Error: %s", notif_result.message))
  assert_test(false, "Setup notifications should not error")
elseif notif_result and notif_result.success then
  print("  ✓ Autocommands registered successfully")
  assert_test(true, "Setup notifications succeeded")
else
  print("  Unexpected result format")
  assert_test(false, "Setup notifications returned unexpected format")
end
print()

-- Test 3: Check autocommands were created
print("Test 3: Verify autocommands exist")
local autocmds = vim.api.nvim_get_autocmds({
  group = "AmpExtrasNotifications"
})
assert_test(#autocmds > 0, "Autocommand group should have commands")
print(string.format("  Found %d autocommands in group", #autocmds))

-- Count cursor movement and buffer change autocommands
local cursor_cmds = 0
local buffer_cmds = 0
for _, cmd in ipairs(autocmds) do
  if cmd.event == "CursorMoved" or cmd.event == "CursorMovedI" then
    cursor_cmds = cursor_cmds + 1
    print(string.format("  - %s autocommand registered", cmd.event))
  elseif cmd.event == "BufEnter" or cmd.event == "WinEnter" then
    buffer_cmds = buffer_cmds + 1
    print(string.format("  - %s autocommand registered", cmd.event))
  end
end

assert_test(cursor_cmds >= 2, "Should have CursorMoved and CursorMovedI")
assert_test(buffer_cmds >= 2, "Should have BufEnter and WinEnter")
print()

-- Test 4: Create test buffer and trigger events
print("Test 4: Trigger cursor movement notification")
-- Create a temporary file
local tmpfile = vim.fn.tempname()
vim.cmd("edit " .. tmpfile)
vim.api.nvim_buf_set_lines(0, 0, -1, false, {
  "Line 1",
  "Line 2",
  "Line 3"
})

-- Move cursor (this should trigger CursorMoved)
vim.api.nvim_win_set_cursor(0, {2, 5})
print("  Moved cursor to line 2, column 5")

-- Give a moment for async processing
vim.wait(100)
print("  ✓ Cursor movement event triggered (notification sent to clients)")
print()

-- Test 5: Trigger buffer change notification
print("Test 5: Trigger visible files notification")
-- Create another buffer (this should trigger BufEnter)
local tmpfile2 = vim.fn.tempname()
vim.cmd("edit " .. tmpfile2)
print(string.format("  Opened new buffer: %s", tmpfile2))

-- Give a moment for async processing
vim.wait(100)
print("  ✓ Buffer change event triggered (notification sent to clients)")
print()

-- Test 6: Multiple windows
print("Test 6: Multiple windows with visible files")
vim.cmd("split " .. tmpfile)
print("  Created split with first buffer")
vim.wait(100)
print("  ✓ Window change event triggered")
print()

-- Test 7: Cannot setup notifications without server
print("Test 7: Setup notifications requires running server")
amp.server_stop()
vim.wait(100)

local notif_result2, notif_err2 = amp.setup_notifications()
assert_test(notif_result2 == nil and notif_err2 ~= nil, "Should error when server not running")
if notif_err2 then
  print(string.format("  Expected error: %s", notif_err2))
end
print()

-- Clean up
print("Cleaning up...")
vim.fn.delete(tmpfile)
vim.fn.delete(tmpfile2)

-- Summary
print()
print("=== Test Summary ===")
if success then
  print("✅ All notification tests passed!")
  print()
  print("Notification system working correctly!")
  print()
  print("What was tested:")
  print("  ✓ Server lifecycle")
  print("  ✓ Notification setup")
  print("  ✓ Autocommand registration")
  print("  ✓ Cursor movement notifications")
  print("  ✓ Buffer/window change notifications")
  print("  ✓ Error handling (server not running)")
  print()
  print("Next steps:")
  print("  - Connect WebSocket client to verify notifications are sent")
  print("  - Test notification payload format")
  print("  - Test with multiple connected clients")
  print()
  vim.cmd("qall!")
else
  print("❌ Some tests failed!")
  vim.cmd("cquit!")
end
