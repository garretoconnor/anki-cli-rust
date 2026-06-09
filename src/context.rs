use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anki::collection::{Collection, CollectionBuilder};
use anki::error::{AnkiError, DbErrorKind};
use anki::prelude::I18n;
use anyhow::{anyhow, bail, Context as _, Result};

pub struct Ctx {
    pub col_path: PathBuf,
    pub json: bool,
    no_backup: bool,
}

impl Ctx {
    pub fn new(flag: Option<PathBuf>, json: bool, no_backup: bool) -> Result<Self> {
        Ok(Self {
            col_path: resolve_collection_path(flag)?,
            json,
            no_backup,
        })
    }

    /// Open the collection for read-only commands (no backup taken).
    pub fn open(&self) -> Result<Collection> {
        open_collection(&self.col_path)
    }

    /// Back up the collection file, then open it for a mutating command.
    pub fn open_for_write(&self) -> Result<Collection> {
        if !self.no_backup {
            let dest = self.backup()?;
            // stderr, so --json output on stdout stays clean
            eprintln!("(backed up collection to {})", dest.display());
        }
        self.open()
    }

    pub fn backup(&self) -> Result<PathBuf> {
        let dir = self
            .col_path
            .parent()
            .ok_or_else(|| anyhow!("collection path has no parent directory"))?
            .join("anki-cli-backups");
        fs::create_dir_all(&dir)
            .with_context(|| format!("creating backup dir {}", dir.display()))?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before 1970")
            .as_secs();
        let dest = dir.join(format!("collection-{stamp}.anki2"));
        fs::copy(&self.col_path, &dest)
            .with_context(|| format!("backing up collection to {}", dest.display()))?;
        Ok(dest)
    }
}

/// Convert an anki-crate Result into an anyhow one with a human-readable
/// message, special-casing the "Anki has the file open" error.
pub fn ank<T>(result: anki::error::Result<T>) -> Result<T> {
    result.map_err(|err| {
        if let AnkiError::DbError { ref source } = err {
            if source.kind == DbErrorKind::Locked {
                return anyhow!(
                    "the collection is locked — quit the Anki app (or wait for a sync to finish) and retry"
                );
            }
        }
        anyhow!(err.message(&I18n::template_only()))
    })
}

fn open_collection(path: &Path) -> Result<Collection> {
    if !path.exists() {
        bail!(
            "no collection file at {} (opening a missing path would create an empty collection; refusing)",
            path.display()
        );
    }
    ank(CollectionBuilder::new(path).build())
        .with_context(|| format!("failed to open {}", path.display()))
}

fn resolve_collection_path(flag: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = flag {
        return Ok(path);
    }
    if let Ok(env) = std::env::var("ANKI_COLLECTION") {
        if !env.is_empty() {
            return Ok(PathBuf::from(env));
        }
    }

    let home = std::env::var("HOME").context("HOME is not set")?;
    let bases = [
        format!("{home}/Library/Application Support/Anki2"), // macOS
        format!("{home}/.local/share/Anki2"),                // Linux
    ];

    let mut found: Vec<PathBuf> = Vec::new();
    for base in &bases {
        let Ok(entries) = fs::read_dir(base) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("collection.anki2");
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }

    match found.len() {
        0 => bail!(
            "no Anki collection found; pass --collection or set $ANKI_COLLECTION to the path of a collection.anki2"
        ),
        1 => Ok(found.remove(0)),
        _ => bail!(
            "multiple Anki profiles found, pick one with --collection or $ANKI_COLLECTION:\n  {}",
            found
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join("\n  ")
        ),
    }
}
