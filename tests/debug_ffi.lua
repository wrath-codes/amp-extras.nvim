-- Debug FFI exports
vim.opt.runtimepath:append(vim.fn.getcwd())

print("=== Debugging FFI Exports ===")
print()

-- Clear package cache to force reload
package.loaded.amp_extras_core = nil

local ffi = require("amp_extras_core")

print("FFI module loaded successfully")
print("Type:", type(ffi))
print()

print("Available functions:")
for k, v in pairs(ffi) do
  print(string.format("  %s: %s", k, type(v)))
end
print()

vim.cmd("qall!")
