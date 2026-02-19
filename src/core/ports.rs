use crate::core::domain::context::Context;
use crate::core::domain::note::Note;
use crate::core::domain::tag::Tag;
use anyhow::Result;

pub trait ContextRepository {
    fn save(&self, context: &Context) -> Result<()>;
    fn get_by_id(&self, id: &str) -> Result<Option<Context>>;
    fn get_all(&self) -> Result<Vec<Context>>;
    fn get_by_name(&self, name: &str) -> Result<Option<Context>>; // This results None if the name does not exists
    fn search_by_name(&self, query: &str) -> Result<Vec<Context>>; // This returns similar Context by name
}

pub trait NoteRepository {
    fn save(&self, note: &Note) -> Result<()>;
    fn get_by_id(&self, id: &str) -> Result<Option<Note>>;
    fn get_by_context(&self, context_id: &str) -> Result<Vec<Note>>;
    fn search_by_tags(&self, query: &[tag]) -> Result<Vec<Note>>;
    fn search_by_content(&self, query: &str) -> Result<Vec<Note>>;
    fn search(&self, query: &str) -> Result<Vec<Note>>; // This search includes content, tags, and context
    fn delete(&self, id: &str) -> Result<()>;
}