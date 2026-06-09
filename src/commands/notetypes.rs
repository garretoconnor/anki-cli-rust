use anki::notetype::Notetype;
use anyhow::{anyhow, Result};
use serde_json::json;

use crate::context::{ank, Ctx};

fn is_cloze(nt: &Notetype) -> bool {
    use anki_proto::notetypes::notetype::config::Kind;
    nt.config.kind() == Kind::Cloze
}

pub fn list(ctx: &Ctx) -> Result<()> {
    let mut col = ctx.open()?;
    let notetypes = ank(col.get_all_notetypes())?;

    if ctx.json {
        let out: Vec<_> = notetypes
            .iter()
            .map(|nt| {
                json!({
                    "id": nt.id.0,
                    "name": nt.name,
                    "kind": if is_cloze(nt) { "cloze" } else { "standard" },
                    "fields": nt.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>(),
                    "templates": nt.templates.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        for nt in &notetypes {
            println!(
                "{:<15} {:<30} fields: {}",
                nt.id.0,
                nt.name,
                nt.fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(())
}

pub fn show(ctx: &Ctx, name: &str) -> Result<()> {
    let mut col = ctx.open()?;
    let nt = ank(col.get_notetype_by_name(name))?
        .ok_or_else(|| anyhow!("no notetype named '{name}'; see `anki-cli notetypes list`"))?;

    if ctx.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "id": nt.id.0,
                "name": nt.name,
                "kind": if is_cloze(&nt) { "cloze" } else { "standard" },
                "fields": nt.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>(),
                "templates": nt.templates.iter().map(|t| json!({
                    "name": t.name,
                    "question": t.config.q_format,
                    "answer": t.config.a_format,
                })).collect::<Vec<_>>(),
            }))?
        );
    } else {
        println!("notetype: {} (id {})", nt.name, nt.id.0);
        println!(
            "kind:     {}",
            if is_cloze(&nt) { "cloze" } else { "standard" }
        );
        println!("fields:");
        for field in &nt.fields {
            println!("  - {}", field.name);
        }
        println!("templates:");
        for tmpl in &nt.templates {
            println!("  - {}", tmpl.name);
            println!("      Q: {}", tmpl.config.q_format.replace('\n', " "));
            println!("      A: {}", tmpl.config.a_format.replace('\n', " "));
        }
    }
    Ok(())
}
