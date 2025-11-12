-- Minimal test to debug plugin loading
vim.opt.runtimepath:append(vim.fn.getcwd())

print("Step 1: Loading plugin...")
local ok, amp = pcall(require, "amp_extras")

if not ok then
  print("ERROR loading plugin: " .. tostring(amp))
  vim.cmd("qall!")
  return
end

print("✅ Plugin loaded")
print("")

print("Step 2: Starting server...")
local result, err = amp.server_start()

if err then
  print("ERROR starting server: " .. err)
  vim.cmd("qall!")
  return
end

if not result then
  print("ERROR: server_start returned nil")
  vim.cmd("qall!")
  return
end

print("✅ Server started!")
print("Port:", result.port)
print("Token:", result.token)
print("Lockfile:", result.lockfile)
print("")

-- Check if lockfile exists
local lockfile_exists = vim.fn.filereadable(result.lockfile) == 1
print("Lockfile exists:", lockfile_exists)

if lockfile_exists then
  print("")
  print("Success! You can now connect Amp CLI with:")
  print(string.format("  amp --ide \"ws://127.0.0.1:%d/?auth=%s\"", result.port, result.token))
end

print("")
print("Press Ctrl+C to stop")

-- Keep running
while true do
  vim.wait(1000)
end
