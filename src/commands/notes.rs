use anki::collection::Collection;
use anki::notes::{Note, NoteId};
use anki::notetype::Notetype;
use anki::search::SortMode;
use anki::services::CardsService;
use anyhow::{anyhow, Result};
use serde_json::json;

use super::{deck_names, field_index, parse_field_args, preview, queue_name};
use crate::context::{ank, Ctx};

fn get_note(col: &mut Collection, id: i64) -> Result<Note> {
    ank(col.storage.get_note(NoteId(id)))?
        .ok_or_else(|| anyhow!("no note with id {id}; see `anki-cli notes find`"))
}

fn notetype_of(col: &mut Collection, note: &Note) -> Result<std::sync::Arc<Notetype>> {
    ank(col.get_notetype(note.notetype_id))?
        .ok_or_else(|| anyhow!("notetype {} missing from collection", note.notetype_id.0))
}

fn fields_json(nt: &Notetype, note: &Note) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (idx, value) in note.fields().iter().enumerate() {
        let name = nt
            .fields
            .get(idx)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| format!("field{idx}"));
        map.insert(name, json!(value));
    }
    serde_json::Value::Object(map)
}

pub fn add(
    ctx: &Ctx,
    deck: &str,
    notetype: &str,
    field_args: &[String],
    tags: Vec<String>,
) -> Result<()> {
    let fields = parse_field_args(field_args)?;
    if fields.is_empty() {
        return Err(anyhow!("at least one --field NAME=VALUE is required"));
    }

    let mut col = ctx.open_for_write()?;
    let nt = ank(col.get_notetype_by_name(notetype))?
        .ok_or_else(|| anyhow!("no notetype named '{notetype}'; see `anki-cli notetypes list`"))?;
    let deck = ank(col.get_or_create_normal_deck(deck))?;

    let mut note = nt.new_note();
    for (name, value) in fields {
        let idx = field_index(&nt, &name)?;
        ank(note.set_field(idx, value))?;
    }
    note.tags = tags;

    let out = ank(col.add_note(&mut note, deck.id))?;

    if ctx.json {
        println!(
            "{}",
            json!({
                "ok": true,
                "noteId": note.id.0,
                "deck": deck.name.human_name(),
                "cardsGenerated": out.output,
            })
        );
    } else {
        println!(
            "added note {} to '{}' ({} card(s) generated)",
            note.id.0,
            deck.name.human_name(),
            out.output
        );
    }
    Ok(())
}

pub fn find(ctx: &Ctx, query: &str, limit: usize) -> Result<()> {
    let mut col = ctx.open()?;
    let nids = ank(col.search_notes(query, SortMode::NoOrder))?;
    let total = nids.len();

    let mut entries = Vec::new();
    for nid in nids.into_iter().take(limit) {
        let note = get_note(&mut col, nid.0)?;
        let nt = notetype_of(&mut col, &note)?;
        entries.push((note, nt));
    }

    if ctx.json {
        let out: Vec<_> = entries
            .iter()
            .map(|(note, nt)| {
                json!({
                    "id": note.id.0,
                    "notetype": nt.name,
                    "tags": note.tags,
                    "fields": fields_json(nt, note),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "total": total, "notes": out }))?
        );
    } else {
        for (note, _) in &entries {
            let first = note.fields().first().map(String::as_str).unwrap_or("");
            let tags = if note.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", note.tags.join(", "))
            };
            println!("{:<15} {}{}", note.id.0, preview(first, 70), tags);
        }
        if total > entries.len() {
            eprintln!(
                "(showing {} of {total} notes; raise --limit)",
                entries.len()
            );
        }
    }
    Ok(())
}

pub fn show(ctx: &Ctx, id: i64) -> Result<()> {
    let mut col = ctx.open()?;
    let note = get_note(&mut col, id)?;
    let nt = notetype_of(&mut col, &note)?;
    let decks = deck_names(&col)?;

    let cards = ank(col.storage.all_cards_of_note(note.id))?;
    let mut card_views = Vec::new();
    for card in &cards {
        let pc = ank(CardsService::get_card(
            &mut col,
            anki_proto::cards::CardId { cid: card.id().0 },
        ))?;
        let deck = decks
            .get(&card.deck_id())
            .cloned()
            .unwrap_or_else(|| format!("deck {}", pc.deck_id));
        card_views.push((pc, deck));
    }

    if ctx.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": note.id.0,
                "guid": note.guid,
                "notetype": nt.name,
                "tags": note.tags,
                "fields": fields_json(&nt, &note),
                "cards": card_views.iter().map(|(pc, deck)| json!({
                    "id": pc.id,
                    "deck": deck,
                    "template": nt.templates.get(pc.template_idx as usize).map(|t| t.name.clone()),
                    "queue": queue_name(pc.queue),
                    "due": pc.due,
                    "intervalDays": pc.interval,
                    "easeFactor": pc.ease_factor as f32 / 1000.0,
                    "reps": pc.reps,
                    "lapses": pc.lapses,
                })).collect::<Vec<_>>(),
            }))?
        );
    } else {
        println!("note {} ({})", note.id.0, nt.name);
        if !note.tags.is_empty() {
            println!("tags: {}", note.tags.join(", "));
        }
        for (idx, value) in note.fields().iter().enumerate() {
            let name = nt.fields.get(idx).map(|f| f.name.as_str()).unwrap_or("?");
            println!("{name}: {value}");
        }
        println!("cards:");
        for (pc, deck) in &card_views {
            println!(
                "  {:<15} deck='{}' queue={} due={} ivl={}d reps={} lapses={}",
                pc.id,
                deck,
                queue_name(pc.queue),
                pc.due,
                pc.interval,
                pc.reps,
                pc.lapses
            );
        }
    }
    Ok(())
}

pub fn edit(
    ctx: &Ctx,
    id: i64,
    field_args: &[String],
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
) -> Result<()> {
    let fields = parse_field_args(field_args)?;
    if fields.is_empty() && add_tags.is_empty() && remove_tags.is_empty() {
        return Err(anyhow!(
            "nothing to do: pass --field, --add-tag, or --remove-tag"
        ));
    }

    let mut col = ctx.open_for_write()?;
    let mut note = get_note(&mut col, id)?;
    let nt = notetype_of(&mut col, &note)?;

    for (name, value) in fields {
        let idx = field_index(&nt, &name)?;
        ank(note.set_field(idx, value))?;
    }
    for tag in add_tags {
        if !note.tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
            note.tags.push(tag);
        }
    }
    note.tags
        .retain(|t| !remove_tags.iter().any(|r| r.eq_ignore_ascii_case(t)));

    ank(col.update_note(&mut note))?;

    if ctx.json {
        println!(
            "{}",
            json!({ "ok": true, "noteId": note.id.0, "tags": note.tags })
        );
    } else {
        println!("updated note {}", note.id.0);
    }
    Ok(())
}

pub fn delete(ctx: &Ctx, ids: &[i64]) -> Result<()> {
    let mut col = ctx.open_for_write()?;
    let nids: Vec<NoteId> = ids.iter().map(|&id| NoteId(id)).collect();
    // fail early on a bad id rather than silently skipping it
    for nid in &nids {
        get_note(&mut col, nid.0)?;
    }
    ank(col.remove_notes(&nids))?;

    if ctx.json {
        println!("{}", json!({ "ok": true, "deletedNotes": ids }));
    } else {
        println!("deleted {} note(s)", ids.len());
    }
    Ok(())
}
