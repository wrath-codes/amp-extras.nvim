-- Headless test for WebSocket server integration
-- Run with: nvim --headless -u tests/server_test.lua

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

print("=== WebSocket Server Integration Tests ===")
print()

-- Test 1: Server should not be running initially
print("Test 1: Initial state")
local running = amp.server_is_running()
assert_test(not running, "Server should not be running initially")
print()

-- Test 2: Start server
print("Test 2: Start server")
local result, err = amp.server_start()
assert_test(result ~= nil, "Server should start successfully")
assert_test(err == nil, "Server start should not return error")

if result then
  assert_test(result.port ~= nil, "Server should return port")
  assert_test(result.token ~= nil, "Server should return token")
  assert_test(result.lockfile ~= nil, "Server should return lockfile path")
  assert_test(string.len(result.token) == 32, "Token should be 32 characters")
  
  print(string.format("  Port: %d", result.port))
  print(string.format("  Token: %s", result.token:sub(1, 8) .. "..."))
  print(string.format("  Lockfile: %s", result.lockfile))
end
print()

-- Test 3: Server should be running
print("Test 3: Server running state")
running = amp.server_is_running()
assert_test(running, "Server should be running after start")
print()

-- Test 4: Cannot start twice
print("Test 4: Cannot start twice")
local result2, err2 = amp.server_start()
assert_test(result2 == nil, "Second start should fail")
assert_test(err2 ~= nil, "Second start should return error")
if err2 then
  print(string.format("  Expected error: %s", err2))
end
print()

-- Test 5: Test ping command (basic FFI test)
print("Test 5: FFI ping command")
local ping_result = amp.call("ping", {})
assert_test(ping_result ~= nil, "Ping should succeed")
assert_test(ping_result.pong == true, "Ping should return pong=true")
print()

-- Test 6: Stop server
print("Test 6: Stop server")
local stopped = amp.server_stop()
assert_test(stopped, "Server should stop successfully")
print()

-- Test 7: Server should not be running after stop
print("Test 7: Server stopped state")
running = amp.server_is_running()
assert_test(not running, "Server should not be running after stop")
print()

-- Test 8: Can start again after stop
print("Test 8: Restart server")
result, err = amp.server_start()
assert_test(result ~= nil, "Server should start again after stop")
assert_test(err == nil, "Restart should not return error")
print()

-- Test 9: Server info display
print("Test 9: Server info")
if result then
  print(string.format("  Port: %d", result.port))
  print(string.format("  Token: %s...", result.token:sub(1, 8)))
  print(string.format("  Lockfile: %s", result.lockfile))
  
  -- Clean up
  amp.server_stop()
  print("  Server stopped")
end
print()

-- Summary
print("=== Test Summary ===")
if success then
  print("✅ All tests passed!")
  print()
  print("Server lifecycle working correctly!")
  print()
  print("To test WebSocket functionality:")
  print("  ./tests/test_websocket.sh <port> <token>")
  print()
  vim.cmd("qall!")
else
  print("❌ Some tests failed!")
  vim.cmd("cquit!")
end
