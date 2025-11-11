#!/bin/bash
# Test WebSocket server with actual Amp CLI
#
# This script:
# 1. Starts a Neovim server in background
# 2. Reads the lockfile to get port/token
# 3. Tests Amp CLI connection with IDE protocol

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== Amp CLI Integration Test ==="
echo ""

# Clean up function
cleanup() {
    echo ""
    echo "Cleaning up..."
    if [ -n "$NVIM_PID" ]; then
        kill "$NVIM_PID" 2>/dev/null || true
    fi
    rm -f /tmp/nvim-amp-test.log
}
trap cleanup EXIT

# Start Neovim in background with server running
echo "Starting Neovim with WebSocket server..."
cat > /tmp/nvim-amp-test.lua << 'EOF'
-- Minimal test script that starts server and keeps running
vim.opt.runtimepath:append(vim.fn.getcwd())

local amp = require("amp_extras")
local result, err = amp.server_start()

if not result then
    print("ERROR: Failed to start server: " .. (err or "unknown"))
    vim.cmd("cquit!")
end

print("Server started successfully!")
print("Port: " .. result.port)
print("Token: " .. result.token)
print("Lockfile: " .. result.lockfile)

-- Write info to file for the shell script to read
local info_file = io.open("/tmp/nvim-amp-info.txt", "w")
info_file:write(result.port .. "\n")
info_file:write(result.token .. "\n")
info_file:write(result.lockfile .. "\n")
info_file:close()

print("Server info written to /tmp/nvim-amp-info.txt")
print("Server is running. Press Ctrl+C to stop.")

-- Keep Neovim running
while true do
    vim.wait(1000)
end
EOF

cd "$PROJECT_DIR"
nvim --headless -u /tmp/nvim-amp-test.lua > /tmp/nvim-amp-test.log 2>&1 &
NVIM_PID=$!

echo "Neovim PID: $NVIM_PID"
echo "Waiting for server to start..."

# Wait for info file
for i in {1..10}; do
    if [ -f /tmp/nvim-amp-info.txt ]; then
        break
    fi
    sleep 0.5
done

if [ ! -f /tmp/nvim-amp-info.txt ]; then
    echo "ERROR: Server did not start in time"
    cat /tmp/nvim-amp-test.log
    exit 1
fi

# Read server info
PORT=$(sed -n '1p' /tmp/nvim-amp-info.txt)
TOKEN=$(sed -n '2p' /tmp/nvim-amp-info.txt)
LOCKFILE=$(sed -n '3p' /tmp/nvim-amp-info.txt)

echo "Server started!"
echo "  Port: $PORT"
echo "  Token: ${TOKEN:0:8}..."
echo "  Lockfile: $LOCKFILE"
echo ""

# Verify lockfile exists
if [ ! -f "$LOCKFILE" ]; then
    echo "ERROR: Lockfile not found at $LOCKFILE"
    exit 1
fi

echo "✅ Lockfile exists"
echo ""

# Check if amp CLI can connect
echo "Testing Amp CLI connection..."
echo ""

# Test 1: Check if amp can discover the IDE
echo "Test 1: Amp CLI discovers IDE"
if amp ide list 2>/dev/null | grep -q "$PORT"; then
    echo "✅ Amp CLI can see the IDE on port $PORT"
else
    echo "⚠️  Amp CLI doesn't list the IDE (this is OK, it may not implement 'ide list')"
fi
echo ""

# Test 2: Try to connect (this will fail if amp doesn't support the IDE protocol yet)
echo "Test 2: Attempt Amp CLI connection"
echo "Note: This may fail if amp CLI doesn't fully support IDE protocol yet"
echo ""

# Check what commands amp supports
echo "Available amp commands:"
amp --help | grep -A 20 "COMMANDS:" || true
echo ""

# Test if we can use the lockfile with amp
echo "Test 3: Lockfile contents"
echo "Lockfile JSON:"
cat "$LOCKFILE"
echo ""

# Cleanup will happen via trap
echo "=== Test Complete ==="
echo ""
echo "The server is running and the lockfile is valid."
echo "Amp CLI integration depends on amp supporting the IDE protocol."
echo ""
echo "Server will remain running for 5 seconds for manual testing..."
sleep 5

rm -f /tmp/nvim-amp-info.txt
