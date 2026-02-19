use serde::{Deserialize, Serialize};

pub struct Tag {
    pub name: String,
}

impl Tag {
    pub fn new(raw_name: &str) -> Self {
        let cleaned = raw_name.trim().to_lowercase().replace(" ", "-");
        Self {
            name: cleaned
        }
    }
}