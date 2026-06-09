use anyhow::{anyhow, Result};
use serde_json::json;

use crate::context::{ank, Ctx};

pub fn add(ctx: &Ctx, query: &str, tags: &[String]) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let nids = ank(col.search_notes_unordered(query))?;
    if nids.is_empty() {
        return Err(anyhow!("no notes matched '{query}'"));
    }
    let out = ank(col.add_tags_to_notes(&nids, &tags.join(" ")))?;

    if ctx.json {
        println!(
            "{}",
            json!({ "ok": true, "matchedNotes": nids.len(), "changedNotes": out.output, "tags": tags })
        );
    } else {
        println!(
            "tagged {} of {} matched note(s) with: {}",
            out.output,
            nids.len(),
            tags.join(", ")
        );
    }
    Ok(())
}

pub fn remove(ctx: &Ctx, query: &str, tags: &[String]) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let nids = ank(col.search_notes_unordered(query))?;
    if nids.is_empty() {
        return Err(anyhow!("no notes matched '{query}'"));
    }
    let out = ank(col.remove_tags_from_notes(&nids, &tags.join(" ")))?;

    if ctx.json {
        println!(
            "{}",
            json!({ "ok": true, "matchedNotes": nids.len(), "changedNotes": out.output, "tags": tags })
        );
    } else {
        println!(
            "removed tags from {} of {} matched note(s): {}",
            out.output,
            nids.len(),
            tags.join(", ")
        );
    }
    Ok(())
}
