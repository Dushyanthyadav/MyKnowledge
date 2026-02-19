use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;


use super::tag::Tag;

pub struct Note {
    pub id: String,
    pub context_id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<Tags>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Note {
    pub fn new(context_id: String, title: String, content: String, raw_tags: Vec<String>) -> Self {
        let tags: Vec<Tag> = raw_tags
            .into_iter()
            .map(|t| Tag::new(&t))
            .collect();

        Self {
            id: Uuid::new_v4().to_string(),
            context_id,
            title: title.trim().to_string(),
            content: content.trim().to_string(),
            tags,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn update_content(&mut self, new_content: &str) {
        self.content = new_content.trim().to_string();
        self.updated_at = Utc::now();
    }

    pub fn update_title(&mut self, new_title: &str) {
        self.title = new_title.trim().to_string();
        self.updated_at = Utc::now();
    }
}