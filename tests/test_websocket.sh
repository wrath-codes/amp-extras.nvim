#!/bin/bash
# WebSocket client test helper
# Usage: ./tests/test_websocket.sh <port> <token>

if [ $# -ne 2 ]; then
  echo "Usage: $0 <port> <token>"
  echo ""
  echo "Example:"
  echo "  1. In Neovim, run: :lua local r = require('amp_extras').server_start(); print('Port:', r.port, 'Token:', r.token)"
  echo "  2. Copy the port and token"
  echo "  3. Run: ./tests/test_websocket.sh 12345 abc123..."
  exit 1
fi

PORT=$1
TOKEN=$2

echo "Testing WebSocket connection to port $PORT"
echo "Token: ${TOKEN:0:8}..."
echo ""

export WS_PORT=$PORT
export WS_TOKEN=$TOKEN

cargo test --test websocket_client -- --nocapture
