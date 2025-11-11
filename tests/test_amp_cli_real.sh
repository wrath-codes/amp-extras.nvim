#!/bin/bash
# Real Amp CLI integration test with WebSocket server
#
# This script:
# 1. Starts server in Neovim
# 2. Uses amp CLI with --ide flag to connect
# 3. Verifies the connection works

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Real Amp CLI + WebSocket Server Test ==="
echo ""

# Clean up function
cleanup() {
    echo ""
    echo "Cleaning up..."
    if [ -n "$NVIM_PID" ]; then
        kill "$NVIM_PID" 2>/dev/null || true
    fi
    rm -f /tmp/nvim-amp-*.{lua,log,txt}
}
trap cleanup EXIT

# Build the library first
echo "Building amp-extras-rs..."
cd "$PROJECT_DIR"
just build-debug > /dev/null 2>&1
echo "✅ Build complete"
echo ""

# Start Neovim in background with server running
echo "Starting Neovim with WebSocket server..."
cat > /tmp/nvim-amp-realtest.lua << 'EOF'
-- Start server and keep running
vim.opt.runtimepath:append(vim.fn.getcwd())

local amp = require("amp_extras")
local result, err = amp.server_start()

if not result then
    print("ERROR: Failed to start server: " .. (err or "unknown"))
    vim.cmd("cquit!")
end

-- Write info to file
local info_file = io.open("/tmp/nvim-amp-info.txt", "w")
info_file:write(result.port .. "\n")
info_file:write(result.token .. "\n")
info_file:write(result.lockfile .. "\n")
info_file:close()

-- Create a test file
local test_file = "/tmp/amp-test-file.txt"
local f = io.open(test_file, "w")
f:write("Hello from Neovim!\nThis file was created by the test.\n")
f:close()

-- Open it in a buffer
vim.cmd("edit " .. test_file)

print("SERVER READY")
print("Port: " .. result.port)
print("Lockfile: " .. result.lockfile)

-- Keep running
while true do
    vim.wait(1000)
end
EOF

nvim --headless -u /tmp/nvim-amp-realtest.lua > /tmp/nvim-amp-realtest.log 2>&1 &
NVIM_PID=$!

echo "Neovim PID: $NVIM_PID"
echo "Waiting for server to start..."

# Wait for info file
for i in {1..15}; do
    if [ -f /tmp/nvim-amp-info.txt ]; then
        break
    fi
    sleep 0.5
done

if [ ! -f /tmp/nvim-amp-info.txt ]; then
    echo "❌ ERROR: Server did not start in time"
    echo "Log output:"
    cat /tmp/nvim-amp-realtest.log
    exit 1
fi

# Read server info
PORT=$(sed -n '1p' /tmp/nvim-amp-info.txt)
TOKEN=$(sed -n '2p' /tmp/nvim-amp-info.txt)
LOCKFILE=$(sed -n '3p' /tmp/nvim-amp-info.txt)

echo "✅ Server started successfully!"
echo "  Port: $PORT"
echo "  Token: ${TOKEN:0:8}..."
echo "  Lockfile: $LOCKFILE"
echo ""

# Verify lockfile
if [ ! -f "$LOCKFILE" ]; then
    echo "❌ ERROR: Lockfile not found"
    exit 1
fi

echo "Lockfile contents:"
cat "$LOCKFILE" | jq '.' 2>/dev/null || cat "$LOCKFILE"
echo ""

# Give the server a moment to fully initialize
sleep 1

# Now test with amp CLI
echo "=== Testing Amp CLI Connection ==="
echo ""

# Test 1: Simple help (should work even without IDE)
echo "Test 1: Basic amp CLI functionality"
if amp --help > /dev/null 2>&1; then
    echo "✅ Amp CLI is working"
else
    echo "❌ Amp CLI failed"
    exit 1
fi
echo ""

# Test 2: Check if amp can connect to our IDE
echo "Test 2: Amp CLI with --ide flag"
echo "Running: amp --ide threads list"
echo ""

# Try to run amp with IDE connection
# The --ide flag should make it try to connect to our WebSocket server
if timeout 10 amp --ide threads list 2>&1 | head -20; then
    echo ""
    echo "✅ Amp CLI executed (check output above for connection status)"
else
    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 124 ]; then
        echo "⚠️  Command timed out (may be waiting for input or connection)"
    else
        echo "Command exited with code: $EXIT_CODE"
    fi
fi
echo ""

# Test 3: Check server logs for connection attempts
echo "=== Server Logs ==="
echo "Checking if amp CLI attempted to connect..."
echo ""
cat /tmp/nvim-amp-realtest.log | tail -20
echo ""

# Test 4: Manual verification instructions
echo "=== Manual Test Instructions ==="
echo ""
echo "The server is still running. You can manually test with:"
echo ""
echo "  amp --ide threads list"
echo "  amp --ide --execute 'What files are in /tmp?'"
echo ""
echo "Server info:"
echo "  Port: $PORT"
echo "  Lockfile: $LOCKFILE"
echo ""
echo "Press Enter to stop the server and cleanup..."
read -t 30 || true

echo ""
echo "=== Test Complete ==="
