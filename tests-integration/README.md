# Integration Tests

nvim-oxi integration tests for amp-extras-rs using real Neovim instances.

## Running Tests

### Quick Tests (16 tests, ~1 second)
```bash
just test-integration
# or
cd tests-integration && cargo test --lib
```

These tests run:
- ✅ Command dispatch (ping, list_commands)
- ✅ Diagnostics collection (all scenarios)
- ✅ URI/path conversions
- ⏭️  Skips WebSocket-dependent tests (5 ignored)

### Full WebSocket Integration Tests (with real Amp CLI)
```bash
just test-integration-full
```

Runs `tests/run_integration_tests.sh` which:
1. Starts Neovim with WebSocket server in headless mode
2. Connects real Amp CLI client
3. Tests server lifecycle, authentication, JSON-RPC, notifications
4. Cleans up processes

## Test Architecture

### Production Tokio Server Testing

The WebSocket server uses Tokio (`tokio-tungstenite`) in production. Integration tests verify:

1. **Unit-level tests** (in tests-integration/): Test Neovim API integration without server
2. **Full integration** (in tests/): Test real Tokio server + real Amp CLI client

### Why Some Tests Are Ignored

The 5 WebSocket-dependent command tests (`test_send_*`) are marked `#[ignore]` because:

- `server::stop()` blocks waiting for Tokio runtime to finish
- The runtime waits on `listener.accept()` which needs a wake-up mechanism
- Full server lifecycle testing requires external client (handled by `test-integration-full`)

These tests still verify:
- Commands exist and can be dispatched
- No panics occur
- Basic validation works

### Test Organization

```
tests-integration/
├── src/
│   ├── commands.rs     # Command dispatch tests
│   ├── diagnostics.rs  # vim.diagnostic integration
│   ├── uri.rs          # Path/URI conversions
│   └── lib.rs          # Test module root
├── build.rs            # nvim-oxi test build script
└── Cargo.toml          # Test dependencies

tests/
├── run_integration_tests.sh  # Full WebSocket test suite
├── automated_websocket_test.lua
└── websocket_integration.rs
```

## Adding New Tests

### nvim-oxi tests (tests-integration/)

```rust
#[nvim_oxi::test]
fn test_my_feature() {
    // Call mark_nvim_ready() if using ide_ops functions
    amp_extras::ide_ops::mark_nvim_ready();
    
    // Use nvim-oxi API directly
    let buf = api::create_buf(true, false).unwrap();
    
    // Test your feature
    let result = amp_extras::commands::dispatch("my_command", json!({}));
    assert!(result.is_ok());
}
```

### WebSocket tests (tests/)

Add to `run_integration_tests.sh` or create new Lua test file.

## CI/CD Integration

```yaml
# GitHub Actions example
- name: Run integration tests
  run: |
    just test-all  # Runs unit + integration (no WebSocket)
    
# Optional: Full WebSocket tests (requires Amp CLI binary)
- name: Run WebSocket integration
  run: just test-integration-full
```
