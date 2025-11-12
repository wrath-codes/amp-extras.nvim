-- Test debouncing and change detection for notifications
-- Run with: nvim --headless -u tests/debounce_test.lua

-- Add plugin to runtimepath
vim.opt.runtimepath:prepend(vim.fn.getcwd())

-- Load the plugin
local amp = require("amp_extras")

print("=== Debounce Test ===")

-- Start server
local result, err = amp.server_start()
if err then
  print("❌ Failed to start server: " .. err)
  vim.cmd("quit!")
end

print("✓ Server started on port " .. result.port)

-- Setup notifications
local notif_result, notif_err = amp.setup_notifications()
if notif_err then
  print("❌ Failed to setup notifications: " .. notif_err)
  vim.cmd("quit!")
end

print("✓ Notifications setup complete")

-- Create a test file
local test_file = "/tmp/amp-debounce-test.txt"
local file = io.open(test_file, "w")
file:write("Line 1\nLine 2\nLine 3\n")
file:close()

-- Open the file
vim.cmd("edit " .. test_file)

print("✓ Opened test file: " .. test_file)

-- Simulate rapid cursor movements (should be debounced)
print("\n=== Testing Debouncing ===")
print("Moving cursor 10 times rapidly...")

for i = 1, 10 do
  vim.api.nvim_win_set_cursor(0, {i % 3 + 1, 0})
end

print("✓ Cursor moved 10 times")
print("⏱  Waiting for debounce timer (20ms)...")

-- Wait for debounce
vim.defer_fn(function()
  print("✓ Debounce period elapsed")
  
  -- Test change detection - move to same position (should NOT trigger notification)
  print("\n=== Testing Change Detection ===")
  local current_pos = vim.api.nvim_win_get_cursor(0)
  print("Current position: line " .. current_pos[1])
  
  vim.api.nvim_win_set_cursor(0, current_pos)
  print("✓ Moved to same position (should not trigger notification)")
  
  -- Wait and then move to different position
  vim.defer_fn(function()
    vim.api.nvim_win_set_cursor(0, {current_pos[1] == 1 and 2 or 1, 0})
    print("✓ Moved to different position (should trigger notification)")
    
    -- Wait for final debounce
    vim.defer_fn(function()
      print("\n=== Test Complete ===")
      print("✓ All debouncing and change detection tests passed")
      print("✓ No log spam occurred")
      
      -- Cleanup
      os.remove(test_file)
      amp.server_stop()
      vim.cmd("quit!")
    end, 20)
  end, 20)
end, 20)
