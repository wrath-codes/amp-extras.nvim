# amp-extras-rs

A complete Rust rewrite of [amp-extras.nvim](https://github.com/wrath-codes/amp-extras.nvim) - A fully-featured Neovim plugin for the [Amp CLI](https://ampcode.com).

## Status

🚧 **Under Active Development** - Phase 0 (Setup & Planning)

## Features (Planned)

- **Thread Management** - List, search, archive, and manage Amp threads
- **Prompt Library** - Store and organize reusable prompts with SQLite
- **Permission Editor** - Visual editor for Amp tool permissions
- **MCP Integration** - Manage MCP servers and tools
- **Rich UI** - Built entirely with nui-components (no external dependencies)
- **Lightning Fast** - <1ms SQLite queries, local-first architecture
- **Offline Capable** - Works without network connection

## Architecture

- **Backend:** Rust with [nvim-oxi](https://github.com/noib3/nvim-oxi) (FFI-based)
- **Frontend:** Lua with [nui-components](https://github.com/grapp-dev/nui-components.nvim)
- **Database:** SQLite with WAL mode
- **Thread Storage:** Hybrid approach (local files + CLI)

## Requirements

- Neovim 0.9+
- Rust 1.75+ (stable)
- Amp CLI installed and configured
- Lua dependencies:
  - `nui-components.nvim`
  - `nui.nvim`

## Installation (Not Yet Ready)

```lua
-- lazy.nvim
{
  "wrath-codes/amp-extras-rs",
  dependencies = {
    "grapp-dev/nui-components.nvim",
  },
  build = "just build",
  config = function()
    require("amp_extras").setup({
      -- Configuration options
    })
  end,
}
```

## Development

### Build

```bash
# Build the project
just build

# Run tests
just test

# Format code
just fmt

# Run linter
just lint

# Install to Neovim
just install
```

### Project Structure

```
amp-extras-rs/
├── crates/
│   └── core/          # Rust backend
├── lua/
│   └── amp_extras/    # Lua frontend
├── plugin/            # Neovim plugin entry point
├── schemas/           # JSON schemas
└── docs/              # Documentation
```

## License

MIT
