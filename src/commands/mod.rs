pub mod cards;
pub mod decks;
pub mod notes;
pub mod notetypes;
pub mod tags;

use std::collections::HashMap;

use anki::collection::Collection;
use anki::decks::DeckId;
use anki::notetype::Notetype;
use anyhow::{anyhow, Result};

use crate::context::ank;

/// Map deck ids to human-readable names ("Parent::Child").
pub fn deck_names(col: &Collection) -> Result<HashMap<DeckId, String>> {
    Ok(ank(col.get_all_deck_names(false))?.into_iter().collect())
}

/// Resolve a field name to its ordinal, case-insensitively.
pub fn field_index(nt: &Notetype, name: &str) -> Result<usize> {
    nt.fields
        .iter()
        .position(|f| f.name == name)
        .or_else(|| {
            nt.fields
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(name))
        })
        .ok_or_else(|| {
            anyhow!(
                "notetype '{}' has no field '{}' (fields: {})",
                nt.name,
                name,
                nt.fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Parse a repeatable NAME=VALUE argument.
pub fn parse_field_args(args: &[String]) -> Result<Vec<(String, String)>> {
    args.iter()
        .map(|arg| {
            arg.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| anyhow!("invalid --field '{arg}', expected NAME=VALUE"))
        })
        .collect()
}

/// One-line plain-text preview of a (possibly HTML) field value.
pub fn preview(html: &str, max_chars: usize) -> String {
    let text = anki::text::strip_html(html).replace('\n', " ");
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push('…');
    }
    out
}

/// Human name for a card queue value (see CardQueue in rslib).
pub fn queue_name(queue: i32) -> &'static str {
    match queue {
        0 => "new",
        1 => "learn",
        2 => "review",
        3 => "daylearn",
        4 => "preview",
        -1 => "suspended",
        -2 | -3 => "buried",
        _ => "unknown",
    }
}
