// src/main.rs

mod adapters;
mod core;

use anyhow::{Context as AnyhowContext, Result};
use clap::{Parser, Subcommand};
use std::io::{self, Read, Write};
use std::path::PathBuf;

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
    /// Switch your active context (e.g., `mk use Coding`)
    Use {
        /// The name of the context you want to switch to
        context_name: String,
    },

    /// Interactively add a new Note to your active context
    Add,

    /// Manage your Contexts (Buckets)
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },

    /// List notes (Defaults to your active context)
    List {
        /// Filter by a specific context (e.g., -c Rust)
        #[arg(short, long)]
        context: Option<String>,

        /// Show notes from ALL contexts
        #[arg(short, long)]
        all: bool,
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
    /// Read the full content of a note
    Read {
        /// The UUID of the note you want to read
        id: String,
    },
    /// Export all notes to a single Markdown file for backup
    Export {
        /// The file path to save the backup
        #[arg(short, long, default_value = "myknowledge_backup.md")]
        output: String,
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
    /// Show the currently active context
    Active,
}

// --- STATE MANAGEMENT HELPERS ---
fn set_active_context(base_path: &PathBuf, context_id: &str) -> Result<()> {
    std::fs::write(base_path.join(".active_context"), context_id)?;
    Ok(())
}

fn get_active_context(base_path: &PathBuf) -> Result<Option<String>> {
    let path = base_path.join(".active_context");
    if path.exists() {
        let id = std::fs::read_to_string(path)?;
        Ok(Some(id.trim().to_string()))
    } else {
        Ok(None)
    }
}

// --- INTERACTIVE PROMPT HELPER ---
fn prompt_user(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}

// --- MULTILINE PROMPT HELPER ---
fn prompt_multiline(prompt: &str) -> Result<String> {
    println!("{}", prompt);
    println!("(Type your note. Press Ctrl+D (Mac/Linux) or Ctrl+Z (Windows) on a new line to save.)");
    
    let mut full_text = String::new();
    // read_to_string keeps recording until it receives the EOF signal
    io::stdin().read_to_string(&mut full_text)?;
    
    // We trim the end to remove any trailing whitespace or accidental newlines
    Ok(full_text.trim().to_string())
}

// --- PRINT HELPER ---
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
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let base_path = home_dir.join(".myknowledge");

    if !base_path.exists() {
        std::fs::create_dir_all(&base_path)?;
    }

    let context_repo = FileContextRepository::new(&base_path);
    let note_repo = FileNoteRepository::new(&base_path);
    let cli = Cli::parse();

    match cli.command {
        // --- SWITCH ACTIVE CONTEXT ---
        Commands::Use { context_name } => match context_repo.get_by_name(&context_name)? {
            Some(ctx) => {
                set_active_context(&base_path, &ctx.id)?;
                println!("🎯 Active context set to: '{}'", ctx.name);
            }
            None => {
                println!(
                    "❌ Context '{}' not found. Create it with `mk context add \"{}\"`",
                    context_name, context_name
                );
            }
        },

        // --- INTERACTIVE ADD ---
        Commands::Add => {
            let active_id = match get_active_context(&base_path)? {
                Some(id) => id,
                None => {
                    println!("❌ You have no active context set!");
                    println!("Please set one first by running: mk use <ContextName>");
                    return Ok(());
                }
            };

            let active_context = match context_repo.get_by_id(&active_id)? {
                Some(ctx) => ctx,
                None => {
                    println!("❌ Your active context is corrupted. Please run `mk use` again.");
                    return Ok(());
                }
            };

            println!("✍️  Adding note to [{}]", active_context.name);

            let title = prompt_user("Title: ")?;
            if title.is_empty() {
                println!("❌ Title cannot be empty. Aborting.");
                return Ok(());
            }

            let content = prompt_multiline("\nNote:")?;
            
            // Because Ctrl+D closes the standard input stream, we need to add a newline
            // to keep the terminal output looking clean before asking for tags.
            println!();

            let tags_input = prompt_user("Tags (comma separated, or press Enter to skip): ")?;
            let tags: Vec<String> = if tags_input.is_empty() {
                Vec::new()
            } else {
                tags_input
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| s.trim().to_string())
                    .collect()
            };

            let note = Note::new(active_context.id, title.clone(), content, tags);
            note_repo.save(&note)?;

            println!("\n✅ Saved '{}' to [{}]", title, active_context.name);
        }

        // --- CONTEXT COMMANDS ---
        Commands::Context { action } => match action {
            ContextAction::Add { name, description } => {
                let ctx = Context::new(&name, description);
                context_repo.save(&ctx)?;
                println!("✅ Created Context: '{}'", ctx.name);

                set_active_context(&base_path, &ctx.id)?;
                println!("🎯 Active context is now: '{}'", ctx.name);
            }
            ContextAction::List => {
                let active_id = get_active_context(&base_path)?.unwrap_or_default();
                let contexts = context_repo.get_all()?;

                if contexts.is_empty() {
                    println!("No contexts found. Create one with `mk context add <NAME>`");
                } else {
                    println!("📚 Your Contexts:");
                    for c in contexts {
                        let indicator = if c.id == active_id { "⭐" } else { "  " };
                        println!("{} {} (ID: {})", indicator, c.name, c.id);
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
            ContextAction::Active => match get_active_context(&base_path)? {
                Some(id) => match context_repo.get_by_id(&id)? {
                    Some(ctx) => println!("🎯 Current active context: '{}'", ctx.name),
                    None => println!(
                        "❌ Your active context is corrupted. Please run `mk use <Name>` again."
                    ),
                },
                None => {
                    println!("ℹ️  No active context set.");
                    println!("Set one by running: mk use <ContextName>");
                }
            },
        },

       // --- LIST NOTES ---
        Commands::List { context, all } => {
            // 1. Did the user explicitly ask for everything?
            if all {
                println!("📚 All Notes:");
                let notes = note_repo.search("")?;
                print_notes(&notes, &context_repo)?;
                return Ok(());
            }

            // 2. Determine which context to look at
            let target_context_name = if let Some(ctx_name) = context {
                // The user passed a specific flag like `-c Recipes`
                ctx_name
            } else if let Some(active_id) = get_active_context(&base_path)? {
                // The user just typed `mk list`, so we grab the active context!
                match context_repo.get_by_id(&active_id)? {
                    Some(ctx) => ctx.name,
                    None => {
                        println!("❌ Your active context is corrupted. Run `mk use <Name>` to fix it.");
                        return Ok(());
                    }
                }
            } else {
                // Fallback: No active context is set yet
                println!("ℹ️  No active context set. Showing all notes:");
                let notes = note_repo.search("")?;
                print_notes(&notes, &context_repo)?;
                return Ok(());
            };

            // 3. Fetch and print the scoped notes
            match context_repo.get_by_name(&target_context_name)? {
                Some(ctx) => {
                    println!("📚 Notes in Context: [{}]", ctx.name);
                    let notes = note_repo.get_by_context(&ctx.id)?;
                    print_notes(&notes, &context_repo)?;
                }
                None => println!("❌ Error: Context '{}' not found.", target_context_name),
            }
        },

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
        } => match note_repo.get_by_id(&id)? {
            Some(mut note) => {
                let mut was_changed = false;

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
        },

        // --- READ NOTE ---
        Commands::Read { id } => {
            match note_repo.get_by_id(&id)? {
                Some(note) => {
                    // Grab the context name so we can display it nicely
                    let ctx_name = match context_repo.get_by_id(&note.context_id)? {
                        Some(c) => c.name,
                        None => "Unknown Context".to_string(),
                    };
                    
                    let tags_display: Vec<String> = note.tags.iter().map(|t| format!("#{}", t.name)).collect();

                    // Print a beautiful header
                    println!("\n==================================================");
                    println!("📄 {}", note.title);
                    println!("   Context: {} | Tags: {}", ctx_name, tags_display.join(", "));
                    println!("   Updated: {}", note.updated_at.format("%Y-%m-%d %H:%M"));
                    println!("==================================================\n");
                    
                    // Print the actual multi-line content
                    println!("{}\n", note.content);
                }
                None => {
                    println!("❌ Error: No note found with ID: {}", id);
                }
            }
        },

        // --- EXPORT / BACKUP NOTES ---
        Commands::Export { output } => {
            let notes = note_repo.search("")?; // Empty search returns everything!
            
            if notes.is_empty() {
                println!("ℹ️  No notes found to export.");
                return Ok(());
            }

            // Create the physical file
            let mut file = std::fs::File::create(&output)
                .with_context(|| format!("Failed to create export file at '{}'", output))?;

            // Write a nice header
            writeln!(file, "# MyKnowledge Master Backup\n")?;

            // Loop through and format every single note
            for note in &notes {
                let ctx_name = match context_repo.get_by_id(&note.context_id)? {
                    Some(c) => c.name,
                    None => "Unknown Context".to_string(),
                };
                
                let tags_display: Vec<String> = note.tags.iter().map(|t| format!("#{}", t.name)).collect();

                writeln!(file, "## {}", note.title)?;
                writeln!(file, "**Context:** {} | **Tags:** {}", ctx_name, tags_display.join(", "))?;
                writeln!(file, "**ID:** `{}`", note.id)?;
                writeln!(file, "---\n")?;
                writeln!(file, "{}\n", note.content)?;
                writeln!(file, "<br>\n")?; // Adds some spacing between notes
            }

            println!("✅ Successfully backed up {} note(s) to '{}'", notes.len(), output);
        },

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
