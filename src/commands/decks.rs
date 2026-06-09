use anki::search::{SearchNode, SortMode};
use anyhow::{anyhow, Result};
use serde_json::json;

use crate::context::{ank, Ctx};

pub fn list(ctx: &Ctx) -> Result<()> {
    let mut col = ctx.open()?;
    let names = ank(col.get_all_deck_names(false))?;

    let mut rows = Vec::new();
    for (did, name) in names {
        let cards = ank(col.search_cards(
            SearchNode::DeckIdsWithoutChildren(did.0.to_string()),
            SortMode::NoOrder,
        ))?
        .len();
        rows.push((did, name, cards));
    }

    if ctx.json {
        let out: Vec<_> = rows
            .iter()
            .map(|(did, name, cards)| json!({ "id": did.0, "name": name, "cards": cards }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        for (did, name, cards) in &rows {
            println!("{:<15} {:>5}  {}", did.0, cards, name);
        }
    }
    Ok(())
}

pub fn create(ctx: &Ctx, name: &str) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let existed = ank(col.get_deck_id(name))?.is_some();
    let deck = ank(col.get_or_create_normal_deck(name))?;

    if ctx.json {
        println!(
            "{}",
            json!({ "ok": true, "id": deck.id.0, "name": deck.name.human_name(), "created": !existed })
        );
    } else if existed {
        println!("deck '{name}' already exists (id {})", deck.id.0);
    } else {
        println!("created deck '{name}' (id {})", deck.id.0);
    }
    Ok(())
}

pub fn rename(ctx: &Ctx, name: &str, new_name: &str) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let did = ank(col.get_deck_id(name))?
        .ok_or_else(|| anyhow!("no deck named '{name}'; see `anki-cli decks list`"))?;
    ank(col.rename_deck(did, new_name))?;

    if ctx.json {
        println!("{}", json!({ "ok": true, "id": did.0, "name": new_name }));
    } else {
        println!("renamed '{name}' -> '{new_name}'");
    }
    Ok(())
}

pub fn delete(ctx: &Ctx, name: &str) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let did = ank(col.get_deck_id(name))?
        .ok_or_else(|| anyhow!("no deck named '{name}'; see `anki-cli decks list`"))?;
    let out = ank(col.remove_decks_and_child_decks(&[did]))?;

    if ctx.json {
        println!(
            "{}",
            json!({ "ok": true, "deletedDeck": name, "removedCards": out.output })
        );
    } else {
        println!(
            "deleted deck '{name}' and its subdecks ({} card(s) removed)",
            out.output
        );
    }
    Ok(())
}
