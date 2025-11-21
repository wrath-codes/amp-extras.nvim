pub const SCHEMA: &str = "
-- Core prompts table
CREATE TABLE IF NOT EXISTS prompts (
    id TEXT PRIMARY KEY,          -- UUID v4 string
    title TEXT NOT NULL,          -- Display title
    description TEXT,             -- Optional description
    content TEXT NOT NULL,        -- The prompt text
    tags TEXT,                    -- JSON array of strings: [\"code\", \"debug\"]
    usage_count INTEGER DEFAULT 0,-- Increment on use
    last_used_at INTEGER,         -- Unix timestamp (seconds)
    created_at INTEGER NOT NULL,  -- Unix timestamp (seconds)
    updated_at INTEGER NOT NULL   -- Unix timestamp (seconds)
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_prompts_usage ON prompts(usage_count DESC);
CREATE INDEX IF NOT EXISTS idx_prompts_updated ON prompts(updated_at DESC);
";