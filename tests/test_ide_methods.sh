#!/bin/bash
# Test what IDE methods Amp CLI actually calls
#
# This monitors the server logs to see which IDE operations
# Amp CLI invokes when running commands

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== IDE Methods Test ==="
echo ""
echo "This test monitors which IDE protocol methods Amp CLI calls"
echo ""

# Clean up function
cleanup() {
    echo ""
    echo "Cleaning up..."
    if [ -n "$NVIM_PID" ]; then
        kill "$NVIM_PID" 2>/dev/null || true
    fi
    rm -f /tmp/nvim-ide-*.{lua,log,txt}
    rm -f /tmp/test-*.txt
}
trap cleanup EXIT

# Build
cd "$PROJECT_DIR"
just build-debug > /dev/null 2>&1

# Create test files
echo "Creating test files..."
echo "Test content line 1" > /tmp/test-file-1.txt
echo "Test content line 2" > /tmp/test-file-2.txt
echo "✅ Test files created"
echo ""

# Start server with verbose logging
echo "Starting server with logging..."
cat > /tmp/nvim-ide-test.lua << 'EOF'
vim.opt.runtimepath:append(vim.fn.getcwd())

-- Enable verbose output
local original_print = print
_G.print = function(...)
    local args = {...}
    local msg = table.concat(vim.tbl_map(tostring, args), " ")
    -- Write to log file
    local log = io.open("/tmp/nvim-ide-test.log", "a")
    log:write(os.date("%H:%M:%S") .. " " .. msg .. "\n")
    log:close()
    original_print(...)
end

local amp = require("amp_extras")
local result = amp.server_start()

if not result then
    print("ERROR: Server failed to start")
    vim.cmd("cquit!")
end

-- Write server info
local info = io.open("/tmp/nvim-ide-info.txt", "w")
info:write(result.port .. "\n" .. result.token .. "\n")
info:close()

-- Open test files in buffers
vim.cmd("edit /tmp/test-file-1.txt")
vim.cmd("badd /tmp/test-file-2.txt")

print("SERVER READY - Monitoring IDE method calls...")
print("Port: " .. result.port)

-- Keep running
while true do
    vim.wait(1000)
end
EOF

# Clear log
> /tmp/nvim-ide-test.log

nvim --headless -u /tmp/nvim-ide-test.lua > /dev/null 2>&1 &
NVIM_PID=$!

# Wait for server
for i in {1..10}; do
    if [ -f /tmp/nvim-ide-info.txt ]; then break; fi
    sleep 0.5
done

if [ ! -f /tmp/nvim-ide-info.txt ]; then
    echo "❌ Server failed to start"
    exit 1
fi

PORT=$(sed -n '1p' /tmp/nvim-ide-info.txt)
echo "✅ Server running on port $PORT"
echo ""

# Test different amp commands and see what they call
echo "=== Test 1: Simple command (threads list) ==="
amp --ide threads list > /dev/null 2>&1
sleep 1
echo "IDE methods called:"
grep -i "ide\|nvim" /tmp/nvim-ide-test.log 2>/dev/null | tail -5 || echo "  (none detected)"
echo ""

echo "=== Test 2: Execute command with file context ==="
# This should potentially call ide/readFile
echo "Running: amp --ide --execute 'What is in /tmp/test-file-1.txt?'"
timeout 30 amp --ide --execute 'What is in /tmp/test-file-1.txt?' 2>&1 | head -10 &
AMP_PID=$!
sleep 3
kill $AMP_PID 2>/dev/null || true
wait $AMP_PID 2>/dev/null || true

echo ""
echo "IDE methods called:"
grep -i "ide\|nvim" /tmp/nvim-ide-test.log 2>/dev/null | tail -10 || echo "  (none detected)"
echo ""

echo "=== Full Server Log ==="
cat /tmp/nvim-ide-test.log
echo ""

echo "=== Summary ==="
echo "Check the log above to see which IDE methods were called"
echo "Expected methods: ide/ping, ide/readFile, ide/editFile, nvim/notify"
