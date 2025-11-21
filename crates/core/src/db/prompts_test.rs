#[cfg(test)]
mod tests {
    use crate::db::Db;
    use crate::db::prompts::{create_prompt, list_prompts, update_prompt, record_usage, delete_prompt};
    use crate::errors::Result;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_crud_operations() -> Result<()> {
        // Setup isolated DB
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test_prompts.db");
        
        // Initialize DB
        Db::init(db_path.to_str().unwrap()).await?;

        // 1. Create
        let prompt = create_prompt(
            "Test Title".into(),
            Some("Test Description".into()),
            "Test Content".into(),
            Some(vec!["tag1".into(), "tag2".into()])
        ).await?;

        assert_eq!(prompt.title, "Test Title");
        assert_eq!(prompt.description, Some("Test Description".into()));
        assert_eq!(prompt.usage_count, 0);

        // 2. List
        let prompts = list_prompts().await?;
        assert!(!prompts.is_empty());
        assert_eq!(prompts[0].id, prompt.id);

        // 3. Update
        update_prompt(
            prompt.id.clone(),
            "Updated Title".into(),
            Some("Updated Description".into()),
            "Updated Content".into(),
            None
        ).await?;

        let prompts = list_prompts().await?;
        assert_eq!(prompts[0].title, "Updated Title");
        assert_eq!(prompts[0].description, Some("Updated Description".into()));
        assert_eq!(prompts[0].content, "Updated Content");

        // 4. Usage
        record_usage(prompt.id.clone()).await?;
        let prompts = list_prompts().await?;
        assert_eq!(prompts[0].usage_count, 1);

        // 5. Delete
        delete_prompt(prompt.id.clone()).await?;
        let prompts = list_prompts().await?;
        assert!(prompts.iter().all(|p| p.id != prompt.id));
        
        Ok(())
    }
}
