-- Test for userSentMessage and appendToPrompt notifications
-- Run with: nvim --headless -u tests/user_message_test.lua
--
-- This test demonstrates how to send messages from Neovim to Amp CLI

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

print("=== User Message Notification Tests ===")
print()

-- Test 1: Start server
print("Test 1: Start WebSocket server")
local server_result = amp.server_start()
assert_test(server_result ~= nil and server_result.success, "Server should start")

if server_result and server_result.success then
  print(string.format("  ✓ Server started on port %d", server_result.port))
  print(string.format("  ✓ Token: %s...", server_result.token:sub(1, 8)))
  print(string.format("  ✓ Lockfile: %s", server_result.lockfile))
else
  print("❌ Cannot test without server")
  vim.cmd("cquit!")
end
print()

-- Test 2: Send user message (without server running - should fail)
print("Test 2: Send user message without connected clients")
local result1 = amp.send_user_message("Hello from Neovim!")
assert_test(result1 ~= nil and result1.success, "Should send message even without clients")

if result1 and result1.success then
  print("  ✓ Message sent (broadcast to 0 clients)")
else
  print(string.format("  Error: %s", result1 and result1.message or "unknown"))
end
print()

-- Test 3: Send to prompt
print("Test 3: Append text to Amp IDE prompt")
local result2 = amp.send_to_prompt("@file.rs#L10-L20")
assert_test(result2 ~= nil and result2.success, "Should append to prompt")

if result2 and result2.success then
  print("  ✓ Text appended to prompt")
else
  print(string.format("  Error: %s", result2 and result2.message or "unknown"))
end
print()

-- Test 4: Send empty message
print("Test 4: Send empty message (edge case)")
local result3 = amp.send_user_message("")
assert_test(result3 ~= nil and result3.success, "Should handle empty message")

if result3 and result3.success then
  print("  ✓ Empty message sent successfully")
else
  print(string.format("  Error: %s", result3 and result3.message or "unknown"))
end
print()

-- Test 5: Send multiline message
print("Test 5: Send multiline message")
local multiline_msg = [[Explain this code:

function hello()
  print("Hello, world!")
end]]

local result4 = amp.send_user_message(multiline_msg)
assert_test(result4 ~= nil and result4.success, "Should handle multiline message")

if result4 and result4.success then
  print("  ✓ Multiline message sent successfully")
  print("  Message preview:")
  for line in multiline_msg:gmatch("[^\n]+") do
    print("    " .. line)
  end
else
  print(string.format("  Error: %s", result4 and result4.message or "unknown"))
end
print()

-- Test 6: Send to prompt with visual selection reference
print("Test 6: Append file reference to prompt")
local file_ref = string.format("@%s#L%d-L%d", "test.lua", 10, 20)
local result5 = amp.send_to_prompt(file_ref)
assert_test(result5 ~= nil and result5.success, "Should append file reference")

if result5 and result5.success then
  print(string.format("  ✓ File reference appended: %s", file_ref))
else
  print(string.format("  Error: %s", result5 and result5.message or "unknown"))
end
print()

-- Test 7: Error handling - server stopped
print("Test 7: Error when server not running")
amp.server_stop()
vim.wait(100)  -- Give server time to fully stop

local result6, err6 = amp.send_user_message("This should fail")
assert_test(result6 == nil and err6 ~= nil, "Should error when server stopped")

if err6 then
  print(string.format("  ✓ Expected error: %s", err6))
elseif result6 and result6.error then
  print(string.format("  ✓ Expected error: %s", result6.message))
else
  print("  ⚠️  Server may not have stopped in time (async shutdown)")
  print("  Note: This is acceptable - message sent to 0 clients")
end
print()

-- Summary
print("=== Test Summary ===")
if success then
  print("✅ All user message tests passed!")
  print()
  print("FFI functions tested:")
  print("  ✓ send_user_message()")
  print("  ✓ send_to_prompt()")
  print()
  print("Features verified:")
  print("  ✓ Send messages to Amp CLI")
  print("  ✓ Append text to IDE prompt")
  print("  ✓ Handle empty messages")
  print("  ✓ Handle multiline messages")
  print("  ✓ Handle file references")
  print("  ✓ Error handling when server stopped")
  print()
  print("🎯 Next: Connect Amp CLI to verify messages are received!")
  print()
  vim.cmd("qall!")
else
  print("❌ Some tests failed!")
  vim.cmd("cquit!")
end
