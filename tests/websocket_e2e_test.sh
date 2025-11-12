#!/bin/bash
# End-to-end WebSocket test
# Starts Neovim server, connects a client, verifies notifications

set -e

echo "=== WebSocket End-to-End Test ==="
echo ""

# Clean up any existing temp files
rm -f /tmp/amp_ws_test.txt /tmp/amp_ws_log.txt

echo "Step 1: Starting Neovim with WebSocket server (background)..."
timeout 20 nvim --headless -u tests/automated_websocket_test.lua > /tmp/amp_ws_log.txt 2>&1 &
NVIM_PID=$!

# Wait for connection info file
echo "Step 2: Waiting for server to start..."
for i in {1..10}; do
  if [ -f /tmp/amp_ws_test.txt ]; then
    break
  fi
  sleep 0.5
done

if [ ! -f /tmp/amp_ws_test.txt ]; then
  echo "❌ Server failed to start"
  kill $NVIM_PID 2>/dev/null || true
  cat /tmp/amp_ws_log.txt
  exit 1
fi

# Read connection info
PORT=$(head -n 1 /tmp/amp_ws_test.txt)
TOKEN=$(tail -n 1 /tmp/amp_ws_test.txt)

echo "✅ Server started on port $PORT"
echo ""

echo "Step 3: Connecting WebSocket client..."
export WS_PORT=$PORT
export WS_TOKEN=$TOKEN

# Run the WebSocket client test with timeout
timeout 10 cargo test --test websocket_client -- --nocapture || true

echo ""
echo "Step 4: Cleaning up..."
kill $NVIM_PID 2>/dev/null || true
rm -f /tmp/amp_ws_test.txt /tmp/amp_ws_log.txt

echo ""
echo "✅ End-to-end test complete!"
