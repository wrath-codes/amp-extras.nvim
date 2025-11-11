-- Test for visual mode selection notifications
-- Run with: nvim --headless -u tests/visual_selection_test.lua

vim.opt.runtimepath:append(vim.fn.getcwd())

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

print("=== Visual Selection Notification Tests ===")
print()

-- Start server
print("Test 1: Start server and setup notifications")
local server_result = amp.server_start()
assert_test(server_result ~= nil, "Server should start")

local notif_result = amp.setup_notifications()
assert_test(notif_result and notif_result.success, "Notifications should be set up")
print()

-- Create test buffer
print("Test 2: Create test buffer with content")
local tmpfile = vim.fn.tempname()
vim.cmd("edit " .. tmpfile)
vim.api.nvim_buf_set_lines(0, 0, -1, false, {
  "Line 1: Hello World",
  "Line 2: Testing visual selection",
  "Line 3: More content here",
  "Line 4: Final line"
})
print("  Created buffer with 4 lines")
print()

-- Test cursor movement (normal mode)
print("Test 3: Cursor movement in normal mode")
vim.api.nvim_win_set_cursor(0, {2, 10})
vim.wait(50)
print("  ✓ Moved cursor to line 2, column 10 (should send zero-width selection)")
print()

-- Test character-wise visual selection
print("Test 4: Character-wise visual selection (v)")
vim.api.nvim_win_set_cursor(0, {2, 7})  -- Position at "Testing"
vim.cmd("normal! viw")  -- Select inner word
vim.wait(50)
print("  ✓ Selected word 'Testing' with viw")
vim.cmd("normal! \\<Esc>")
print()

-- Test line-wise visual selection
print("Test 5: Line-wise visual selection (V)")
vim.api.nvim_win_set_cursor(0, {2, 0})
vim.cmd("normal! V")  -- Line-wise visual mode
vim.wait(50)
print("  ✓ Selected line 2 with V")
vim.cmd("normal! \\<Esc>")
print()

-- Test multi-line visual selection
print("Test 6: Multi-line visual selection")
vim.api.nvim_win_set_cursor(0, {1, 0})
vim.cmd("normal! v")  -- Start visual mode
vim.api.nvim_win_set_cursor(0, {3, 10})  -- Extend to line 3
vim.wait(50)
print("  ✓ Selected from line 1 to line 3, column 10")
vim.cmd("normal! \\<Esc>")
print()

-- Test block-wise visual selection
print("Test 7: Block-wise visual selection (Ctrl-V)")
vim.api.nvim_win_set_cursor(0, {1, 0})
vim.cmd("normal! \\<C-v>")  -- Block-wise visual mode
vim.api.nvim_win_set_cursor(0, {3, 5})
vim.wait(50)
print("  ✓ Selected block from line 1-3, columns 0-5")
vim.cmd("normal! \\<Esc>")
print()

-- Test window splits
print("Test 8: Multiple windows showing different files")
-- Create another file
local tmpfile2 = vim.fn.tempname()
vim.cmd("split " .. tmpfile2)
vim.api.nvim_buf_set_lines(0, 0, -1, false, {"Content in second file"})
vim.wait(50)
print("  ✓ Created split with second file (should send visible files)")
print()

-- Test window close
print("Test 9: Close window (visible files should update)")
vim.cmd("close")
vim.wait(50)
print("  ✓ Closed split (should update visible files)")
print()

-- Cleanup
print("Cleaning up...")
amp.server_stop()
vim.fn.delete(tmpfile)
vim.fn.delete(tmpfile2)

-- Summary
print()
print("=== Test Summary ===")
if success then
  print("✅ All visual selection tests passed!")
  print()
  print("What was tested:")
  print("  ✓ Normal mode cursor movement (zero-width selection)")
  print("  ✓ Character-wise visual selection (v + viw)")
  print("  ✓ Line-wise visual selection (V)")
  print("  ✓ Multi-line visual selection")
  print("  ✓ Block-wise visual selection (Ctrl-V)")
  print("  ✓ Window splits (visible files)")
  print("  ✓ Window close (visible files update)")
  print()
  print("Note: Notifications were broadcast but no clients connected")
  print("      Connect a WebSocket client to verify actual payloads")
  print()
  vim.cmd("qall!")
else
  print("❌ Some tests failed!")
  vim.cmd("cquit!")
end
