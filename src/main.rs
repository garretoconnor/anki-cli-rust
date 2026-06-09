mod commands;
mod context;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::context::Ctx;

#[derive(Parser)]
#[command(
    name = "anki-cli",
    version,
    about = "Manage an Anki collection from the command line, via Anki's official Rust backend.",
    after_help = "Writes require exclusive access: quit the Anki app first.\n\
                  A copy of the collection file is saved before every write (disable with --no-backup).\n\
                  Search queries use Anki's search syntax, e.g. 'deck:ML tag:weak attention'."
)]
struct Cli {
    /// Path to collection.anki2 (default: $ANKI_COLLECTION, else auto-detect)
    #[arg(long, global = true, value_name = "PATH")]
    collection: Option<PathBuf>,

    /// Emit machine-readable JSON on stdout
    #[arg(long, global = true)]
    json: bool,

    /// Skip the automatic backup taken before any write
    #[arg(long, global = true)]
    no_backup: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List, create, rename, or delete decks
    Decks {
        #[command(subcommand)]
        command: DeckCmd,
    },
    /// Inspect note types and their fields/templates
    Notetypes {
        #[command(subcommand)]
        command: NotetypeCmd,
    },
    /// Add, search, show, edit, or delete notes
    Notes {
        #[command(subcommand)]
        command: NoteCmd,
    },
    /// List, move, suspend, or unsuspend cards
    Cards {
        #[command(subcommand)]
        command: CardCmd,
    },
    /// Bulk-add or bulk-remove tags on notes matched by a search
    Tags {
        #[command(subcommand)]
        command: TagCmd,
    },
    /// Copy the collection file to a timestamped backup
    Backup,
}

#[derive(Subcommand)]
enum DeckCmd {
    /// List all decks with their card counts
    List,
    /// Create a deck; use :: for nesting (parents are created as needed)
    Create { name: String },
    /// Rename a deck (subdecks move with it)
    Rename { name: String, new_name: String },
    /// Delete a deck, its subdecks, and ALL cards inside them
    Delete { name: String },
}

#[derive(Subcommand)]
enum NotetypeCmd {
    /// List all note types
    List,
    /// Show a note type's fields and card templates
    Show { name: String },
}

#[derive(Subcommand)]
enum NoteCmd {
    /// Add a note
    Add {
        /// Target deck (created if missing)
        #[arg(long)]
        deck: String,
        /// Note type, e.g. "Basic" or "Cloze"
        #[arg(long)]
        notetype: String,
        /// Field value as NAME=VALUE (repeat per field)
        #[arg(short, long = "field", value_name = "NAME=VALUE")]
        fields: Vec<String>,
        /// Tag to attach (repeatable)
        #[arg(short, long = "tag", value_name = "TAG")]
        tags: Vec<String>,
    },
    /// Search notes with Anki search syntax
    Find {
        query: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Show a note in full, including its cards
    Show { id: i64 },
    /// Edit a note's fields and/or tags
    Edit {
        id: i64,
        /// Field value as NAME=VALUE (repeat per field)
        #[arg(short, long = "field", value_name = "NAME=VALUE")]
        fields: Vec<String>,
        #[arg(long = "add-tag", value_name = "TAG")]
        add_tags: Vec<String>,
        #[arg(long = "remove-tag", value_name = "TAG")]
        remove_tags: Vec<String>,
    },
    /// Delete notes and all their cards
    Delete {
        #[arg(required = true)]
        ids: Vec<i64>,
    },
}

#[derive(Subcommand)]
enum CardCmd {
    /// List cards matching an Anki search query
    List {
        query: String,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Move cards to an existing deck
    Move {
        #[arg(long)]
        deck: String,
        #[arg(required = true)]
        ids: Vec<i64>,
    },
    /// Suspend cards (excluded from review until unsuspended)
    Suspend {
        #[arg(required = true)]
        ids: Vec<i64>,
    },
    /// Unsuspend (and unbury) cards
    Unsuspend {
        #[arg(required = true)]
        ids: Vec<i64>,
    },
}

#[derive(Subcommand)]
enum TagCmd {
    /// Add tags to every note matching a search query
    Add {
        query: String,
        #[arg(required = true)]
        tags: Vec<String>,
    },
    /// Remove tags from every note matching a search query
    Remove {
        query: String,
        #[arg(required = true)]
        tags: Vec<String>,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let ctx = Ctx::new(cli.collection, cli.json, cli.no_backup)?;

    match cli.command {
        Command::Decks { command } => match command {
            DeckCmd::List => commands::decks::list(&ctx),
            DeckCmd::Create { name } => commands::decks::create(&ctx, &name),
            DeckCmd::Rename { name, new_name } => commands::decks::rename(&ctx, &name, &new_name),
            DeckCmd::Delete { name } => commands::decks::delete(&ctx, &name),
        },
        Command::Notetypes { command } => match command {
            NotetypeCmd::List => commands::notetypes::list(&ctx),
            NotetypeCmd::Show { name } => commands::notetypes::show(&ctx, &name),
        },
        Command::Notes { command } => match command {
            NoteCmd::Add {
                deck,
                notetype,
                fields,
                tags,
            } => commands::notes::add(&ctx, &deck, &notetype, &fields, tags),
            NoteCmd::Find { query, limit } => commands::notes::find(&ctx, &query, limit),
            NoteCmd::Show { id } => commands::notes::show(&ctx, id),
            NoteCmd::Edit {
                id,
                fields,
                add_tags,
                remove_tags,
            } => commands::notes::edit(&ctx, id, &fields, add_tags, remove_tags),
            NoteCmd::Delete { ids } => commands::notes::delete(&ctx, &ids),
        },
        Command::Cards { command } => match command {
            CardCmd::List { query, limit } => commands::cards::list(&ctx, &query, limit),
            CardCmd::Move { deck, ids } => commands::cards::move_to_deck(&ctx, &deck, &ids),
            CardCmd::Suspend { ids } => commands::cards::suspend(&ctx, &ids),
            CardCmd::Unsuspend { ids } => commands::cards::unsuspend(&ctx, &ids),
        },
        Command::Tags { command } => match command {
            TagCmd::Add { query, tags } => commands::tags::add(&ctx, &query, &tags),
            TagCmd::Remove { query, tags } => commands::tags::remove(&ctx, &query, &tags),
        },
        Command::Backup => {
            let dest = ctx.backup()?;
            if ctx.json {
                println!(
                    "{}",
                    serde_json::json!({ "ok": true, "backup": dest.display().to_string() })
                );
            } else {
                println!("backed up to {}", dest.display());
            }
            Ok(())
        }
    }
}
