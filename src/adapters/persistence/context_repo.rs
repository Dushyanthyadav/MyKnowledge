use anyhow::{Context as _, Result};
use std::fs;
use std::path::PathBuf;

use crate::core::domain::context::Context;
use crate::core::ports::ContextRepository;

// Adapter: A struct that knows how to talk to the file system
pub struct FileContextRepository {
    storage_path: PathBuf,
}

impl FileContextRepository {
    pub fn new(base_path: &PathBuf) -> Self {
        let storage_path = base_path.join("contexts.json");
        Self { storage_path }
    }

    fn load_data(&self) -> Result<Vec<Context>> {
        if !self.storage_path.exists() {
            return Ok(Vec::new()); // Returns empty Vector in first run or the file is not present
        }

        let content =
            fs::read_to_string(&self.storage_path).context("Failed to read contexts.json file")?;

        let contexts: Vec<Context> =
            serde_json::from_str(&content).context("Failed to parse contexts.json")?;

        Ok(contexts)
    }

    fn save_data(&self, contexts: &[Context]) -> Result<()> {
        let json =
            serde_json::to_string_pretty(contexts).context("Failed to serialize contexts")?;

        fs::write(&self.storage_path, json).context("Failed to write to contexts.json")?;

        Ok(())
    }
}

// The port implementation for this adaptor
impl ContextRepository for FileContextRepository {
    fn save(&self, context: &Context) -> Result<()> {
        let mut contexts = self.load_data()?;

        // This is to check if the given context is updated or just new one
        if let Some(index) = contexts.iter().position(|c| c.id == context.id) {
            contexts[index] = context.clone();
        } else {
            contexts.push(context.clone());
        }
        self.save_data(&contexts)
    }

    fn get_by_id(&self, id: &str) -> Result<Option<Context>> {
        let contexts = self.load_data()?;
        Ok(contexts.into_iter().find(|c| c.id == id))
    }

    fn get_all(&self) -> Result<Vec<Context>> {
        self.load_data()
    }

    fn get_by_name(&self, name: &str) -> Result<Option<Context>> {
        let contexts = self.load_data()?;
        let target = name.to_lowercase();

        Ok(contexts
            .into_iter()
            .find(|c| c.name.to_lowercase() == target))
    }

    fn search_by_name(&self, query: &str) -> Result<Vec<Context>> {
        let contexts = self.load_data()?;
        let target = query.to_lowercase();

        let matches: Vec<Context> = contexts
            .into_iter()
            .filter(|c| c.name.to_lowercase().contains(&target))
            .collect();

        Ok(matches)
    }
}
