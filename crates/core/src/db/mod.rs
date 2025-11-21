use std::sync::OnceLock;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use crate::errors::Result;

pub mod schema;
pub mod prompts;
#[cfg(test)]
mod prompts_test;

static DB_POOL: OnceLock<SqlitePool> = OnceLock::new();

pub struct Db;

impl Db {
    /// Initialize the database connection pool
    pub async fn init(path: &str) -> Result<()> {
        if DB_POOL.get().is_some() {
            return Ok(());
        }

        // Create directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(path).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| anyhow::anyhow!("Failed to create database directory: {}", e))?;
        }

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(
                sqlx::sqlite::SqliteConnectOptions::new()
                    .filename(path)
                    .create_if_missing(true)
                    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            )
            .await?;

        // Run schema migration
        // Split by semicolon to run multiple statements
        for statement in schema::SCHEMA.split(';') {
            if statement.trim().is_empty() {
                continue;
            }
            sqlx::query(statement)
                .execute(&pool)
                .await?;
        }

        // Manual migrations
        // Attempt to add description column if it doesn't exist
        let _ = sqlx::query("ALTER TABLE prompts ADD COLUMN description TEXT")
            .execute(&pool)
            .await;

        DB_POOL.set(pool).map_err(|_| anyhow::anyhow!("Failed to set global DB pool"))?;
        
        Ok(())
    }

    /// Get a reference to the global connection pool
    pub fn pool() -> Result<&'static SqlitePool> {
        DB_POOL.get().ok_or_else(|| anyhow::anyhow!("Database not initialized").into())
    }
}
