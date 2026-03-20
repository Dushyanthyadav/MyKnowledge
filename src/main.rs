// src/main.rs

mod adapters;
mod core;

use anyhow::{Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};

use crate::adapters::persistence::context_repo::FileContextRepository;
use crate::adapters::persistence::note_repo::FileNoteRepository;
use crate::core::domain::context::Context;
use crate::core::domain::note::Note;
use crate::core::domain::tag::Tag;
use crate::core::ports::{ContextRepository, NoteRepository};

/// MyKnowledge (mk) - The blazing fast local note taking tool
#[derive(Parser)]
#[command(
    name = "mk",
    version = "1.0",
    about = "Frictionless local knowledge base"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage your Contexts (Buckets)
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    /// Add a new Note
    Add {
        /// The title of the note
        title: String,
        /// The actual content
        content: String,
        /// The name of the context (bucket) to put this in
        #[arg(short, long)]
        context: String,
        /// Tags to attach (can be used multiple times: -t rust -t architecture)
        #[arg(short, long = "tag")]
        tags: Vec<String>,
    },

    /// List notes (Optionally filter by a context)
    List {
        /// Only show notes from this specific context
        #[arg(short, long)]
        context: Option<String>,
    },

    /// Search your notes with high precision
    Search {
        /// Global search query (checks title, content, and tags)
        query: Option<String>,

        /// Search strictly within the content body
        #[arg(short, long)]
        content: Option<String>,

        /// Search strictly by specific tags (can be used multiple times)
        #[arg(short, long = "tag")]
        tags: Vec<String>,
    },

    /// Edit an existing note
    Edit {
        /// The UUID of the note you want to edit
        id: String,

        /// The new title (optional)
        #[arg(long)]
        title: Option<String>,

        /// The new content (optional)
        #[arg(long)]
        content: Option<String>,

        /// Replace existing tags with these new ones (optional)
        #[arg(short, long = "tag")]
        tags: Option<Vec<String>>,
    },

    /// Delete a note by its ID
    Delete {
        /// The UUID of the note you want to delete
        id: String,
    },
}

#[derive(Subcommand)]
enum ContextAction {
    /// Create a new Context bucket
    Add {
        name: String,
        description: Option<String>,
    },
    /// List all existing Contexts
    List,
    /// Fuzzy search for a context by name
    Search { query: String },
}

// --- HELPER FUNCTION TO KEEP CODE DRY ---
fn print_notes(notes: &[Note], context_repo: &FileContextRepository) -> Result<()> {
    if notes.is_empty() {
        println!("No notes found.");
        return Ok(());
    }

    println!("Found {} note(s):\n", notes.len());
    for note in notes {
        let ctx_name = match context_repo.get_by_id(&note.context_id)? {
            Some(c) => c.name,
            None => "Unknown Context".to_string(),
        };

        let tags_display: Vec<String> = note.tags.iter().map(|t| format!("#{}", t.name)).collect();
        // Create a single-line preview of the content
        let preview = note
            .content
            .chars()
            .take(60)
            .collect::<String>()
            .replace('\n', " ");

        println!("📄 {}  [ID: {}]", note.title, note.id);
        println!(
            "   Context: {} | Tags: {}",
            ctx_name,
            tags_display.join(", ")
        );
        println!("   Preview: {}...\n", preview);
    }
    Ok(())
}

fn main() -> Result<()> {
    // 1. Setup Storage Path (~/.myknowledge)
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let base_path = home_dir.join(".myknowledge");

    if !base_path.exists() {
        std::fs::create_dir_all(&base_path)?;
    }

    // 2. Initialize our Adapters (The "Plugs")
    let context_repo = FileContextRepository::new(&base_path);
    let note_repo = FileNoteRepository::new(&base_path);

    // 3. Parse Terminal Commands
    let cli = Cli::parse();

    // 4. Route the Command to the Core Domain
    match cli.command {
        // --- CONTEXT COMMANDS ---
        Commands::Context { action } => match action {
            ContextAction::Add { name, description } => {
                let ctx = Context::new(&name, description);
                context_repo.save(&ctx)?;
                println!("✅ Created new Context: '{}' (ID: {})", ctx.name, ctx.id);
            }
            ContextAction::List => {
                let contexts = context_repo.get_all()?;
                if contexts.is_empty() {
                    println!("No contexts found. Create one with `mk context add <NAME>`");
                } else {
                    println!("📚 Your Contexts:");
                    for c in contexts {
                        println!("  - {} (ID: {})", c.name, c.id);
                    }
                }
            }
            ContextAction::Search { query } => {
                println!("🔍 Searching Contexts for '{}'...", query);
                let contexts = context_repo.search_by_name(&query)?;
                if contexts.is_empty() {
                    println!("No matching contexts found.");
                } else {
                    for c in contexts {
                        println!("  - {} (ID: {})", c.name, c.id);
                    }
                }
            }
        },

        // --- ADD NOTE ---
        Commands::Add {
            title,
            content,
            context,
            tags,
        } => match context_repo.get_by_name(&context)? {
            Some(ctx) => {
                let note = Note::new(ctx.id, title.clone(), content, tags);
                note_repo.save(&note)?;
                println!("✅ Saved Note: '{}' to Context '{}'", title, ctx.name);
            }
            None => {
                println!("❌ Error: Context '{}' not found.", context);
                println!("Create it first using: mk context add \"{}\"", context);
            }
        },

        // --- LIST NOTES ---
        Commands::List { context } => {
            match context {
                Some(ctx_name) => match context_repo.get_by_name(&ctx_name)? {
                    Some(ctx) => {
                        println!("📚 Notes in Context: {}", ctx.name);
                        let notes = note_repo.get_by_context(&ctx.id)?;
                        print_notes(&notes, &context_repo)?;
                    }
                    None => println!("❌ Error: Context '{}' not found.", ctx_name),
                },
                None => {
                    println!("📚 All Notes:");
                    let notes = note_repo.search("")?; // Empty query returns all notes
                    print_notes(&notes, &context_repo)?;
                }
            }
        }

        // --- SEARCH NOTES ---
        Commands::Search {
            query,
            content,
            tags,
        } => {
            let results = if !tags.is_empty() {
                println!("🔍 Searching by Tags: {:?}", tags);
                let domain_tags: Vec<Tag> = tags.iter().map(|t| Tag::new(t)).collect();
                note_repo.search_by_tags(&domain_tags)?
            } else if let Some(c) = content {
                println!("🔍 Searching Content for: '{}'", c);
                note_repo.search_by_content(&c)?
            } else if let Some(q) = query {
                println!("🔍 Global Search for: '{}'", q);
                note_repo.search(&q)?
            } else {
                println!("❌ Error: Please provide a search query. See `mk search --help`");
                return Ok(());
            };

            print_notes(&results, &context_repo)?;
        }

        // --- EDIT NOTE ---
        Commands::Edit {
            id,
            title,
            content,
            tags,
        } => {
            match note_repo.get_by_id(&id)? {
                Some(mut note) => {
                    let mut was_changed = false;

                    // Using Domain Methods to respect Domain-Driven Design
                    if let Some(new_title) = title {
                        note.update_title(&new_title);
                        was_changed = true;
                    }
                    if let Some(new_content) = content {
                        note.update_content(&new_content);
                        was_changed = true;
                    }
                    if let Some(new_tags) = tags {
                        let domain_tags: Vec<Tag> = new_tags.iter().map(|t| Tag::new(t)).collect();
                        note.update_tags(domain_tags);
                        was_changed = true;
                    }

                    if was_changed {
                        note_repo.save(&note)?;
                        println!("✅ Successfully updated note: '{}'", note.title);
                    } else {
                        println!("⚠️ No changes provided. Note remains unchanged.");
                    }
                }
                None => {
                    println!("❌ Error: No note found with ID: {}", id);
                }
            }
        }

        // --- DELETE NOTE ---
        Commands::Delete { id } => {
            if note_repo.get_by_id(&id)?.is_some() {
                note_repo.delete(&id)?;
                println!("🗑️  Successfully deleted note: {}", id);
            } else {
                println!("❌ Error: No note found with ID: {}", id);
            }
        }
    }

    Ok(())
}
