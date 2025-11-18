# amp-extras-rs justfile

# Use rustup's cargo (respects rust-toolchain.toml)
export PATH := env_var('HOME') + "/.cargo/bin:" + env_var('PATH')

# Default recipe (shows available commands)
default:
    @just --list

# Build the Rust library and copy to lua/amp_extras/
build:
    @echo "Building amp-extras-rs..."
    cargo build --release
    @echo "Copying library to lua/amp_extras/..."
    @mkdir -p lua/amp_extras
    @if [ -f target/release/libamp_extras_core.dylib ]; then \
        cp target/release/libamp_extras_core.dylib lua/amp_extras/amp_extras_core.so; \
        echo "✓ Copied libamp_extras_core.dylib -> lua/amp_extras/amp_extras_core.so"; \
    elif [ -f target/release/libamp_extras_core.so ]; then \
        cp target/release/libamp_extras_core.so lua/amp_extras/amp_extras_core.so; \
        echo "✓ Copied libamp_extras_core.so -> lua/amp_extras/amp_extras_core.so"; \
    elif [ -f target/release/amp_extras_core.dll ]; then \
        cp target/release/amp_extras_core.dll lua/amp_extras/amp_extras_core.so; \
        echo "✓ Copied amp_extras_core.dll -> lua/amp_extras/amp_extras_core.so"; \
    else \
        echo "✗ No library found in target/release/"; \
        exit 1; \
    fi
    @echo "✓ Build complete!"

# Build in debug mode (faster compilation)
build-debug:
    @echo "Building amp-extras-rs (debug mode)..."
    cargo build
    @echo "Copying library to lua/amp_extras/..."
    @mkdir -p lua/amp_extras
    @if [ -f target/debug/libamp_extras_core.dylib ]; then \
        cp target/debug/libamp_extras_core.dylib lua/amp_extras/amp_extras_core.so; \
        echo "✓ Copied libamp_extras_core.dylib -> lua/amp_extras/amp_extras_core.so"; \
    elif [ -f target/debug/libamp_extras_core.so ]; then \
        cp target/debug/libamp_extras_core.so lua/amp_extras/amp_extras_core.so; \
        echo "✓ Copied libamp_extras_core.so -> lua/amp_extras/amp_extras_core.so"; \
    elif [ -f target/debug/amp_extras_core.dll ]; then \
        cp target/debug/amp_extras_core.dll lua/amp_extras/amp_extras_core.so; \
        echo "✓ Copied amp_extras_core.dll -> lua/amp_extras/amp_extras_core.so"; \
    else \
        echo "✗ No library found in target/debug/"; \
        exit 1; \
    fi
    @echo "✓ Debug build complete!"

# Run unit tests only (Note: may have teardown issues when running all together)
test:
    @echo "Running unit tests..."
    cargo test --package amp_extras_core --lib
    @echo "✓ Unit tests passed!"

# Run tests with output
test-verbose:
    cargo test --package amp_extras_core --lib -- --nocapture

# Run unit tests by module (avoids global state issues)
test-unit-safe:
    @echo "Running unit tests by module..."
    @cargo test --package amp_extras_core --lib ide_ops::
    @cargo test --package amp_extras_core --lib server::
    @cargo test --package amp_extras_core --lib commands::
    @cargo test --package amp_extras_core --lib notifications::
    @cargo test --package amp_extras_core --lib errors::
    @cargo test --package amp_extras_core --lib rpc::
    @cargo test --package amp_extras_core --lib lockfile::
    @echo "✓ Unit tests passed!"
    @echo "Note: nvim:: module tests are in test-integration (require Neovim)"

# Run nvim-oxi integration tests
test-integration:
    @echo "Running nvim-oxi integration tests..."
    cargo test --package amp-extras-tests
    @echo "✓ Integration tests complete!"

# Run legacy Lua integration tests with Neovim headless
test-integration-lua: build-debug
    @echo "Running legacy Lua integration tests (Neovim headless)..."
    @if command -v nvim >/dev/null 2>&1; then \
        nvim --headless -u tests/server_test.lua || true; \
        echo "✓ Integration tests complete!"; \
    else \
        echo "✗ nvim not found"; \
        echo "  Install Neovim to run integration tests"; \
        exit 1; \
    fi

# Run notification integration tests
test-notifications: build-debug
    @echo "Running notification integration tests (Neovim headless)..."
    @if command -v nvim >/dev/null 2>&1; then \
        nvim --headless -u tests/notification_test.lua || true; \
        echo "✓ Notification tests complete!"; \
    else \
        echo "✗ nvim not found"; \
        echo "  Install Neovim to run integration tests"; \
        exit 1; \
    fi

# Run visual selection notification tests
test-visual: build-debug
    @echo "Running visual selection tests (Neovim headless)..."
    @if command -v nvim >/dev/null 2>&1; then \
        nvim --headless -u tests/visual_selection_test.lua || true; \
        echo "✓ Visual selection tests complete!"; \
    else \
        echo "✗ nvim not found"; \
        echo "  Install Neovim to run integration tests"; \
        exit 1; \
    fi

# Start WebSocket server for manual testing
test-ws-server: build-debug
    @echo "Starting WebSocket test server..."
    @echo "This will start Neovim with the server running."
    @echo "In another terminal, use the displayed command to connect a client."
    @echo ""
    @nvim -u tests/automated_websocket_test.lua

# Run full integration test suite
test-integration-full: build-debug
    @./tests/run_integration_tests.sh

# Run all tests (unit + integration) - uses safe unit test runner
test-all:
    @echo "Running all tests..."
    @just test-unit-safe
    @just test-integration
    @echo "✓ All tests passed!"

# Format all code (Rust + Lua)
fmt:
    @echo "Formatting Rust code..."
    cargo fmt --all
    @echo "Formatting Lua code..."
    @if command -v stylua >/dev/null 2>&1; then \
        stylua lua/ plugin/; \
        echo "✓ Code formatted!"; \
    else \
        echo "⚠ stylua not found, skipping Lua formatting"; \
        echo "  Install with: cargo install stylua"; \
    fi

# Run clippy linter
lint:
    @echo "Running clippy..."
    cargo clippy --workspace --all-targets -- -D warnings
    @echo "✓ No lint warnings!"

# Run clippy with auto-fix
lint-fix:
    @echo "Running clippy with auto-fix..."
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Install to Neovim data directory
install: build
    @echo "Installing to Neovim..."
    @NVIM_DATA_DIR=$$(nvim --headless -c 'echo stdpath("data")' -c 'quit' 2>&1 | tail -n 1); \
    PLUGIN_DIR="$$NVIM_DATA_DIR/lazy/amp-extras-rs"; \
    echo "Installing to: $$PLUGIN_DIR"; \
    mkdir -p "$$PLUGIN_DIR"; \
    cp -r lua "$$PLUGIN_DIR/"; \
    cp -r plugin "$$PLUGIN_DIR/"; \
    echo "✓ Installed to Neovim!"

# Clean build artifacts
clean:
    @echo "Cleaning build artifacts..."
    cargo clean
    rm -f lua/amp_extras/amp_extras_core.so
    rm -f lua/amp_extras_core.so
    @echo "✓ Clean complete!"

# Check for common issues
check:
    @echo "Running cargo check..."
    cargo check --workspace --all-targets
    @echo "✓ Check passed!"

# Build documentation
doc:
    @echo "Building documentation..."
    cargo doc --workspace --no-deps --open

# Run benchmarks
bench:
    @echo "Running benchmarks..."
    cargo bench --workspace

# Watch for changes and rebuild
watch:
    @echo "Watching for changes..."
    @if command -v cargo-watch >/dev/null 2>&1; then \
        cargo watch -x 'build --release' -s 'just build'; \
    else \
        echo "✗ cargo-watch not found"; \
        echo "  Install with: cargo install cargo-watch"; \
        exit 1; \
    fi

# Show project statistics
stats:
    @echo "Project Statistics:"
    @echo "=================="
    @echo "Rust files:"
    @find crates -name '*.rs' | wc -l | xargs echo "  "
    @echo "Lua files:"
    @find lua plugin -name '*.lua' 2>/dev/null | wc -l | xargs echo "  "
    @echo ""
    @echo "Lines of Rust:"
    @find crates -name '*.rs' -exec wc -l {} + | tail -n 1 | awk '{print "  " $1}'
    @echo "Lines of Lua:"
    @find lua plugin -name '*.lua' -exec wc -l {} + 2>/dev/null | tail -n 1 | awk '{print "  " $1}'
