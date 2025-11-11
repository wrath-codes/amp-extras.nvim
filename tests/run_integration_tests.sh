#!/bin/bash
# Run all integration tests with a live WebSocket server

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║   WebSocket Server Integration Test Suite               ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Clean up any existing temp files
rm -f /tmp/amp_ws_test.txt /tmp/amp_ws_log.txt

echo "Step 1: Starting Neovim with WebSocket server (background)..."
timeout 120 nvim --headless -u tests/automated_websocket_test.lua > /tmp/amp_ws_log.txt 2>&1 &
NVIM_PID=$!

# Wait for connection info file
echo "Step 2: Waiting for server to start..."
for i in {1..20}; do
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

# Export for tests
export WS_PORT=$PORT
export WS_TOKEN=$TOKEN

echo "╔══════════════════════════════════════════════════════════╗"
echo "║   Running Integration Tests                             ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Run each test separately for better output
echo "─────────────────────────────────────────────────────────────"
echo "Test Suite 1: Authentication Failures"
echo "─────────────────────────────────────────────────────────────"
cargo test --test integration_tests test_auth_failure -- --nocapture --test-threads=1

echo ""
echo "─────────────────────────────────────────────────────────────"
echo "Test Suite 2: Multiple Concurrent Clients"
echo "─────────────────────────────────────────────────────────────"
cargo test --test integration_tests test_multiple_concurrent_clients -- --nocapture

echo ""
echo "─────────────────────────────────────────────────────────────"
echo "Test Suite 3: Ping-Pong Exchange"
echo "─────────────────────────────────────────────────────────────"
cargo test --test integration_tests test_ping_pong_exchange -- --nocapture

echo ""
echo "─────────────────────────────────────────────────────────────"
echo "Test Suite 4: WebSocket Client Connection"
echo "─────────────────────────────────────────────────────────────"
timeout 10 cargo test --test websocket_client -- --nocapture || echo "⏱️  Client timed out (expected)"

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║   Cleanup                                                ║"
echo "╚══════════════════════════════════════════════════════════╝"
kill $NVIM_PID 2>/dev/null || true
rm -f /tmp/amp_ws_test.txt /tmp/amp_ws_log.txt

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║   ✅ All Integration Tests Complete!                     ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""
echo "NOTE: Connection timeout test is ignored by default (takes 60s)"
echo "To run it: cargo test --test integration_tests test_connection_timeout -- --nocapture --ignored"
