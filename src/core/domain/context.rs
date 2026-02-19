use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub id: String, 
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Context {
    pub fn new(name: &str, decription: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.trim().to_string(),
            decription,
            created_at: Utc::now(),
        }
    }
}

