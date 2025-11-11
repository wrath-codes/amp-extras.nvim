-- Check server status and client count
vim.opt.runtimepath:append(vim.fn.getcwd())

local amp = require("amp_extras")

print("=== Server Status Check ===")
print("")

local running = amp.server_is_running()
print("Server running:", running)

if running then
  print("")
  print("To check for connected clients, look at the Broadcast messages")
  print("when you move your cursor or change files.")
  print("")
  print("If you see 'Broadcast message to 0 clients', it means:")
  print("  1. No Amp CLI is connected yet, OR")
  print("  2. Amp CLI connected to a different server (amp.nvim)")
  print("")
  print("Check lockfiles:")
  print("  ls ~/.local/share/amp/ide/*.json")
  print("")
  print("If you see multiple lockfiles, Amp CLI might be connecting")
  print("to the wrong one (amp.nvim instead of amp-extras-rs)")
  print("")
  print("Solution: Disable amp.nvim plugin temporarily to test amp-extras-rs")
end

vim.cmd("qall!")
