use anyhow::{Context as _, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::domain::note::Note;
use crate::core::domain::tag::Tag;
use crate::core::ports::NoteRepository;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Private struct to handle the YAML at the top of the file
#[derive(Serialize, Deserialize)]
struct NoteFrontmatter {
    id: String,
    context_id: String,
    title: String,
    tags: Vec<Tag>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

// Adapter which works the md file
pub struct FileNoteRepository {
    storage_path: PathBuf,
}

impl FileNoteRepository {
    pub fn new(base_path: &PathBuf) -> Self {
        let storage_path = base_path.join("notes");
        if !storage_path.exists() {
            fs::create_dir_all(&storage_path).expect("Failed to create notes directory");
        }
        Self { storage_path }
    }

    fn get_file_path(&self, id: &str) -> PathBuf {
        self.storage_path.join(format!("{}.md", id))
    }

    ///PRIVATE HELPER
    /// reads a .md file, splits the YAML from the Markdown, and builds a note
    fn parse_note_file(path: &Path) -> Result<Note> {
        let file_content =
            fs::read_to_string(path).with_context(|| format!("Failed to read file: {:?}", path))?;

        // Split the file by the "---" markers
        let parts: Vec<&str> = file_content.splitn(3, "---").collect();

        if parts.len() < 3 {
            anyhow::bail!("Invalid note format (missing frontmatter) in {:?}", path);
        }

        let yaml_str = parts[1];
        let markdown_content = parts[2].trim_start();

        let frontmatter: NoteFrontmatter =
            serde_yaml::from_str(yaml_str).context("Failed to parse YAML frontmatter")?;

        Ok(Note {
            id: frontmatter.id,
            context_id: frontmatter.context_id,
            title: frontmatter.title,
            content: markdown_content.to_string(),
            tags: frontmatter.tags,
            created_at: frontmatter.created_at,
            updated_at: frontmatter.updated_at,
        })
    }

    /// PRIVATE HELPER
    fn get_all_notes(&self) -> Result<Vec<Note>> {
        let mut notes = Vec::new();

        for entry in fs::read_dir(&self.storage_path)? {
            let entry = entry?;
            let path = entry.path();

            // Only process .md files
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                match Self::parse_note_file(&path) {
                    Ok(note) => notes.push(note),
                    Err(e) => eprintln!("Warning: Skipping corrupted note {:?}: {}", path, e),
                }
            }
        }
        Ok(notes)
    }
}

impl NoteRepository for FileNoteRepository {
    fn save(&self, note: &Note) -> Result<()> {
        let path = self.get_file_path(&note.id);

        let frontmatter = NoteFrontmatter {
            id: note.id.clone(),
            context_id: note.context_id.clone(),
            title: note.title.clone(),
            tags: note.tags.clone(),
            created_at: note.created_at,
            updated_at: note.updated_at,
        };

        let yaml_string = serde_yaml::to_string(&frontmatter)?;
        let final_file_content = format!("---\n{}---\n{}", yaml_string, note.content);

        fs::write(&path, final_file_content)?;
        Ok(())
    }

    fn get_by_id(&self, id: &str) -> Result<Option<Note>> {
        let path = self.get_file_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let note = Self::parse_note_file(&path)?;
        Ok(Some(note))
    }

    fn get_by_context(&self, context_id: &str) -> Result<Vec<Note>> {
        let notes = self.get_all_notes()?;
        Ok(notes
            .into_iter()
            .filter(|n| n.context_id == context_id)
            .collect())
    }

    fn search_by_tags(&self, query_tags: &[Tag]) -> Result<Vec<Note>> {
        let notes = self.get_all_notes()?;

        // Return note if ANY of its tags match ANY of the query tags
        Ok(notes
            .into_iter()
            .filter(|n| n.tags.iter().any(|note_tag| query_tags.contains(note_tag)))
            .collect())
    }

    fn search_by_content(&self, query: &str) -> Result<Vec<Note>> {
        let notes = self.get_all_notes()?;
        let query_lower = query.to_lowercase();

        Ok(notes
            .into_iter()
            .filter(|n| n.content.to_lowercase().contains(&query_lower))
            .collect())
    }

    fn search(&self, query: &str) -> Result<Vec<Note>> {
        let notes = self.get_all_notes()?;
        let q = query.to_lowercase();

        // Global search: Checks Title, Content, AND Tags
        Ok(notes
            .into_iter()
            .filter(|n| {
                n.title.to_lowercase().contains(&q)
                    || n.content.to_lowercase().contains(&q)
                    || n.tags.iter().any(|t| t.name.to_lowercase().contains(&q))
            })
            .collect())
    }

    fn delete(&self, id: &str) -> Result<()> {
        let path = self.get_file_path(id);
        if path.exists() {
            fs::remove_file(path).context("Failed to delete note file")?;
        }
        Ok(())
    }
}
