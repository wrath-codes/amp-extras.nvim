use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::Utc;
use crate::errors::Result;
use super::Db;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Prompt {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub content: String,
    pub tags: Option<String>,
    pub usage_count: i32,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub async fn list_prompts() -> Result<Vec<Prompt>> {
    let pool = Db::pool()?;
    let prompts = sqlx::query_as::<_, Prompt>(
        "SELECT * FROM prompts ORDER BY updated_at DESC"
    )
    .fetch_all(pool)
    .await?;
    
    Ok(prompts)
}

pub async fn create_prompt(title: String, description: Option<String>, content: String, tags: Option<Vec<String>>) -> Result<Prompt> {
    let pool = Db::pool()?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp();
    
    let tags_json = tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    sqlx::query(
        "INSERT INTO prompts (id, title, description, content, tags, usage_count, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?)"
    )
    .bind(&id)
    .bind(&title)
    .bind(&description)
    .bind(&content)
    .bind(&tags_json)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(Prompt {
        id,
        title,
        description,
        content,
        tags: tags_json,
        usage_count: 0,
        last_used_at: None,
        created_at: now,
        updated_at: now,
    })
}

pub async fn update_prompt(id: String, title: String, description: Option<String>, content: String, tags: Option<Vec<String>>) -> Result<()> {
    let pool = Db::pool()?;
    let now = Utc::now().timestamp();
    let tags_json = tags.map(|t| serde_json::to_string(&t).unwrap_or_default());

    sqlx::query(
        "UPDATE prompts SET title = ?, description = ?, content = ?, tags = ?, updated_at = ? WHERE id = ?"
    )
    .bind(title)
    .bind(description)
    .bind(content)
    .bind(tags_json)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_prompt(id: String) -> Result<()> {
    let pool = Db::pool()?;
    sqlx::query("DELETE FROM prompts WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn record_usage(id: String) -> Result<()> {
    let pool = Db::pool()?;
    let now = Utc::now().timestamp();
    
    sqlx::query(
        "UPDATE prompts SET usage_count = usage_count + 1, last_used_at = ? WHERE id = ?"
    )
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(())
}