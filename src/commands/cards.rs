use anki::card::CardId;
use anki::search::SortMode;
use anki::services::CardsService;
use anki_proto::scheduler::bury_or_suspend_cards_request::Mode as BuryOrSuspendMode;
use anyhow::{anyhow, Result};
use serde_json::json;

use super::{deck_names, preview, queue_name};
use crate::context::{ank, Ctx};

pub fn list(ctx: &Ctx, query: &str, limit: usize) -> Result<()> {
    let mut col = ctx.open()?;
    let cids = ank(col.search_cards(query, SortMode::NoOrder))?;
    let total = cids.len();
    let decks = deck_names(&col)?;

    let mut rows = Vec::new();
    for cid in cids.into_iter().take(limit) {
        let pc = ank(CardsService::get_card(
            &mut col,
            anki_proto::cards::CardId { cid: cid.0 },
        ))?;
        let first_field = ank(col.storage.get_note(pc.note_id.into()))?
            .and_then(|n| n.fields().first().cloned())
            .unwrap_or_default();
        let deck = decks
            .get(&pc.deck_id.into())
            .cloned()
            .unwrap_or_else(|| format!("deck {}", pc.deck_id));
        rows.push((pc, deck, first_field));
    }

    if ctx.json {
        let out: Vec<_> = rows
            .iter()
            .map(|(pc, deck, first)| {
                json!({
                    "id": pc.id,
                    "noteId": pc.note_id,
                    "deck": deck,
                    "queue": queue_name(pc.queue),
                    "due": pc.due,
                    "intervalDays": pc.interval,
                    "easeFactor": pc.ease_factor as f32 / 1000.0,
                    "reps": pc.reps,
                    "lapses": pc.lapses,
                    "preview": preview(first, 100),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "total": total, "cards": out }))?
        );
    } else {
        for (pc, deck, first) in &rows {
            println!(
                "{:<15} {:<10} ivl={:<5} reps={:<4} {:<25} {}",
                pc.id,
                queue_name(pc.queue),
                format!("{}d", pc.interval),
                pc.reps,
                deck,
                preview(first, 50),
            );
        }
        if total > rows.len() {
            eprintln!("(showing {} of {total} cards; raise --limit)", rows.len());
        }
    }
    Ok(())
}

pub fn move_to_deck(ctx: &Ctx, deck: &str, ids: &[i64]) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let did = ank(col.get_deck_id(deck))?.ok_or_else(|| {
        anyhow!("no deck named '{deck}'; create it first with `anki-cli decks create`")
    })?;
    let cids: Vec<CardId> = ids.iter().map(|&id| CardId(id)).collect();
    let out = ank(col.set_deck(&cids, did))?;

    if ctx.json {
        println!(
            "{}",
            json!({ "ok": true, "deck": deck, "movedCards": out.output })
        );
    } else {
        println!("moved {} card(s) to '{deck}'", out.output);
    }
    Ok(())
}

pub fn suspend(ctx: &Ctx, ids: &[i64]) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let cids: Vec<CardId> = ids.iter().map(|&id| CardId(id)).collect();
    let out = ank(col.bury_or_suspend_cards(&cids, BuryOrSuspendMode::Suspend))?;

    if ctx.json {
        println!("{}", json!({ "ok": true, "suspendedCards": out.output }));
    } else {
        println!("suspended {} card(s)", out.output);
    }
    Ok(())
}

pub fn unsuspend(ctx: &Ctx, ids: &[i64]) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let cids: Vec<CardId> = ids.iter().map(|&id| CardId(id)).collect();
    ank(col.unbury_or_unsuspend_cards(&cids))?;

    if ctx.json {
        println!("{}", json!({ "ok": true, "unsuspendedCards": ids }));
    } else {
        println!("unsuspended {} card(s)", ids.len());
    }
    Ok(())
}
