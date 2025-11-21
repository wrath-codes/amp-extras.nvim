use serde_json::{json, Value};
use crate::{errors::Result, db::prompts, runtime};

pub fn list(_args: Value) -> Result<Value> {
    let prompts = runtime::block_on(async {
        prompts::list_prompts().await
    })?;
    Ok(json!({ "prompts": prompts }))
}

pub fn create(args: Value) -> Result<Value> {
    let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?;
    let description = args.get("description").and_then(|v| v.as_str()).map(String::from);
    let content = args.get("content").and_then(|v| v.as_str()).ok_or("Missing content")?;
    let tags = args.get("tags").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    let prompt = runtime::block_on(async {
        prompts::create_prompt(title.to_string(), description, content.to_string(), tags).await
    })?;

    Ok(json!(prompt))
}

pub fn update(args: Value) -> Result<Value> {
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
    let title = args.get("title").and_then(|v| v.as_str()).ok_or("Missing title")?;
    let description = args.get("description").and_then(|v| v.as_str()).map(String::from);
    let content = args.get("content").and_then(|v| v.as_str()).ok_or("Missing content")?;
    let tags = args.get("tags").and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

    runtime::block_on(async {
        prompts::update_prompt(id.to_string(), title.to_string(), description, content.to_string(), tags).await
    })?;

    Ok(json!({ "success": true }))
}

pub fn delete(args: Value) -> Result<Value> {
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?;
    
    runtime::block_on(async {
        prompts::delete_prompt(id.to_string()).await
    })?;

    Ok(json!({ "success": true }))
}

pub fn use_prompt(args: Value) -> Result<Value> {
    let id = args.get("id").and_then(|v| v.as_str()).ok_or("Missing id")?.to_string();
    
    // Fire and forget
    runtime::spawn(async move {
        if let Err(e) = prompts::record_usage(id).await {
            eprintln!("Failed to record usage: {}", e);
        }
    });

    Ok(json!({ "success": true, "background": true }))
}
