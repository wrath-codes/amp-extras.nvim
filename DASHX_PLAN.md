# DashX (Prompt Library) Detailed Implementation Plan

This document outlines the comprehensive plan to re-implement the **DashX** feature (Prompt Library) in `amp-extras.nvim`.

## 🎯 Goal
Create a fast, persistent library for LLM prompts that allows users to:
1.  **Save** prompts with titles and metadata.
2.  **Search** and filter prompts via a UI.
3.  **Execute** prompts directly into Amp.
4.  **Track** usage statistics.

## 🛠 Tech Stack
*   **Database**: SQLite (via `sqlx` crate) with WAL mode.
*   **Async Runtime**: `tokio` (already present).
*   **Frontend UI**: `nui-components` (Lua).
*   **Bridge**: `nvim-oxi` (Rust FFI).

---

## 🏗 Architecture

### 1. Database Schema
We will use a single `prompts` table in SQLite.

```sql
-- Core prompts table
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,          -- UUID v4 string
    title TEXT NOT NULL,          -- Display title
    content TEXT NOT NULL,        -- The prompt text
    tags TEXT,                    -- JSON array of strings: ["code", "debug"]
    usage_count INTEGER DEFAULT 0,-- Increment on use
    last_used_at INTEGER,         -- Unix timestamp (seconds)
    created_at INTEGER NOT NULL,  -- Unix timestamp (seconds)
    updated_at INTEGER NOT NULL   -- Unix timestamp (seconds)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_prompts_usage ON prompts(usage_count DESC);
CREATE INDEX IF NOT EXISTS idx_prompts_updated ON prompts(updated_at DESC);
```

### 2. Rust Module Structure (`crates/core`)

```text
crates/core/src/
├── db/
│   ├── mod.rs           # Exports and public interface
│   ├── schema.rs        # Schema definition and migration logic
│   └── prompts.rs       # CRUD operations
├── commands/
│   └── prompts.rs       # FFI wrappers handling JSON <-> Rust types
```

### 3. Async Strategy
Neovim's FFI is synchronous. `sqlx` is async.
*   **Global Runtime**: We utilize the existing `crate::runtime::RUNTIME`.
*   **Reads (UI Blocking)**: We use `runtime::block_on` because the UI *cannot* render until it has the data.
*   **Writes (UI Blocking)**: We use `runtime::block_on` to ensure data integrity before closing the modal.
*   **Background (Analytics)**: We use `runtime::spawn` for `record_usage` so typing flow isn't interrupted.

---

## 📋 Detailed Phase Breakdown

### Phase 1: Rust Backend Core

#### 1.1 Dependencies (`crates/core/Cargo.toml`)
Replace `rusqlite` with:
```toml
[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "uuid", "chrono"] }
```

#### 1.2 Database Initialization (`src/db/mod.rs`)
We need a global connection pool accessible to all commands.
```rust
pub struct Db {
    pool: sqlx::SqlitePool,
}

// Singleton instance
static DB: OnceLock<Db> = OnceLock::new();

impl Db {
    // Initialize pool, enable WAL, run migrations
    pub async fn init(path: &str) -> Result<Self>;
}
```

#### 1.3 CRUD Operations (`src/db/prompts.rs`)
Implement the `Prompt` struct and methods:
*   `struct Prompt { id, title, content, tags, usage_count, ... }`
*   `async fn list_prompts(&self) -> Result<Vec<Prompt>>`
    *   Query: `SELECT * FROM prompts ORDER BY updated_at DESC`
*   `async fn create_prompt(&self, title, content, tags) -> Result<Prompt>`
    *   Generates UUID.
    *   Sets `created_at`, `updated_at` to `Utc::now()`.
*   `async fn update_prompt(&self, id, title, content, tags) -> Result<()>`
*   `async fn delete_prompt(&self, id) -> Result<()>`
*   `async fn record_usage(&self, id) -> Result<()>`
    *   Query: `UPDATE prompts SET usage_count = usage_count + 1, last_used_at = ? WHERE id = ?`

#### 1.4 Command Registry (`src/commands/prompts.rs`)
Expose these functions to Lua via `nvim-oxi`. All accept `serde_json::Value` and return `Result<Value>`.

*   `prompts.list`: Calls `db.list_prompts()`.
*   `prompts.create`: Validates input -> `db.create_prompt()`.
*   `prompts.update`: Validates input -> `db.update_prompt()`.
*   `prompts.delete`: `db.delete_prompt()`.
*   `prompts.use`: Calls `db.record_usage()` (spawned async) and returns success immediately.

---

### Phase 2: Lua UI Implementation

#### 2.1 Picker UI (`lua/amp_extras/commands/ui/dashx/picker.lua`)

We will use the **Select** component (which is a wrapper around `Tree`) inside a **Columns** layout to achieve the split-view effect.

**Layout Architecture:**
```lua
local n = require("nui-components")
local renderer = n.create_renderer({ width = 120, height = 30 })

-- Top-level Layout
n.columns(
  -- Left Pane: List of Prompts (30% width)
  n.rows(
    n.text_input({
        id = "search_input",
        placeholder = "Search...",
        on_change = function(value) ... end -- Triggers filter on Select
    }),
    n.select({
        id = "prompt_list",
        data = prompts_data, -- [{id, text=title, content=...}]
        multiselect = false,
        on_select = function(node) ... end, -- Enter key: Execute prompt
        on_change = function(node) ... end, -- Selection move: Update preview
        -- Custom rendering to show usage stats
        prepare_node = function(is_selected, node)
            -- Render "Title (Used: N)"
        end
    })
  ),
  
  -- Right Pane: Preview (70% width)
  n.buffer({
      id = "preview_buffer",
      filetype = "markdown",
      buf_options = { modifiable = false },
      border_label = "Preview"
  })
)
```

**Key Bindings:**
The `Select` component handles arrow keys. We will attach custom mappings to the renderer or the specific component buffer for:
*   `<C-n>`: Trigger `Add Prompt` modal.
*   `<C-e>`: Trigger `Edit Prompt` modal (using `node.id`).
*   `<C-d>`: Trigger `Delete` confirmation.
*   `<Enter>`: Send to Amp (default `on_select` behavior).

#### 2.2 Editor UI (`lua/amp_extras/commands/ui/dashx/edit.lua`)

We will use the **Form** component to handle state and validation automatically.

**Layout Architecture:**
```lua
local n = require("nui-components")

n.form({
    submit_key = "<C-s>", -- Custom save key
    on_submit = function(is_valid)
        if is_valid then
             -- Collect values and call Rust
        end
    end
}, 
  n.rows(
      -- Title Input
      n.text_input({
          id = "title",
          label = "Title",
          placeholder = "e.g. Code Refactor",
          validate = n.validator.min_length(3)
      }),
      
      -- Tags Input
      n.text_input({
          id = "tags",
          label = "Tags (comma separated)",
          placeholder = "rust, logic"
      }),

      -- Content Input (Text Area)
      n.text_input({
          id = "content",
          label = "Prompt Content",
          max_lines = 15, -- Makes it act like a textarea
          autoresize = true,
          linebreak = true
      })
  )
)
```

---

### Phase 3: Integration & Polish

#### 3.1 Migration Script
*   Check for `~/.config/amp-extras/dashx.json` (old format).
*   If exists and DB is empty:
    *   Read JSON.
    *   Loop through items and insert into SQLite.
    *   Rename JSON file to `.bak`.

#### 3.2 Keybindings
In `lua/amp_extras/init.lua`:
*   `:AmpDashX` -> Opens picker.
*   Optional: `<Leader>ax` (Agent Extras) mapping?

#### 3.3 Defaults
If the DB is brand new, populate it with 3 starter prompts:
1.  **Explain Code**: "Explain the selected code in detail..."
2.  **Find Bugs**: "Analyze the following code for potential bugs..."
3.  **Unit Tests**: "Write comprehensive unit tests for..."
