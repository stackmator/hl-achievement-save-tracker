//! Extract every `EnemyDefinition` from the game's `PhoenixGameData.sqlite`
//! along with its real English display name from the `MAIN-enUS.bin`
//! AVAFDICT localization dictionary, both read straight out of the game pak.

use anyhow::{Context, Result};
use hl_save_tracker::avafdict::Dictionary;
use hl_save_tracker::pak::{self, Pak};
use rusqlite::Connection;
use std::io::Write;

struct Enemy {
    id: String,
    spawnable: i64,
    killed_action: Option<String>,
    tip_keys: Vec<String>,
}

fn usage() -> ! {
    eprintln!("usage: enemy_names <pak> [out-file]");
    std::process::exit(2);
}

fn clean(s: &str) -> String {
    // collapse whitespace/newlines so a table row stays on one line
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let pak_path = &args[1];

    let mut pak = Pak::open(pak_path).context("open pak")?;
    eprintln!("pak contains {} indexed files", pak.file_count());

    // 1. Game database (EnemyDefinition lives here).
    let db_entry = pak.find(pak::sqlite_db_path()).cloned().ok_or_else(|| {
        anyhow::anyhow!("{} not in pak", pak::sqlite_db_path())
    })?;
    let db_bytes = pak.read(&db_entry)?;
    eprintln!("extracted {} sqlite bytes", db_bytes.len());

    // 2. English localization dictionary (enemy display names).
    let loc_entry = pak
        .find(pak::en_locres_path())
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("{} not in pak", pak::en_locres_path()))?;
    let loc_bytes = pak.read(&loc_entry)?;
    eprintln!("extracted {} locres bytes", loc_bytes.len());
    let dict = Dictionary::parse(&loc_bytes).context("parse AVAFDICT")?;
    eprintln!("AVAFDICT: {} key/value pairs", dict.len());

    // 3. Load the sqlite db from a temp file (rusqlite needs a path).
    let tmp = std::env::temp_dir().join("hl_enemy_names.sqlite");
    std::fs::write(&tmp, &db_bytes).context("write temp sqlite")?;
    let conn = Connection::open(&tmp).context("open extracted sqlite")?;

    // 4. Pull every enemy plus its knowledge-page hint keys.
    let mut stmt = conn
        .prepare(
            "SELECT EnemyID, IsSpawnable, KilledKnowledgeAction, \
             Attribute1, Attribute2, Attribute3 FROM EnemyDefinition ORDER BY EnemyID",
        )
        .context("query EnemyDefinition")?;
    let mut enemies = Vec::new();
    {
        let rows = stmt
            .query_map([], |r| {
                let id: String = r.get(0)?;
                let spawnable: i64 = r.get(1)?;
                let killed: Option<String> = r.get(2)?;
                let mut tips = Vec::new();
                for c in [3, 4, 5] {
                    if let Some(k) = r.get::<_, Option<String>>(c)? {
                        if !k.is_empty() {
                            tips.push(k);
                        }
                    }
                }
                Ok(Enemy {
                    id,
                    spawnable,
                    killed_action: killed,
                    tip_keys: tips,
                })
            })
            .context("iterate EnemyDefinition")?;
        for e in rows {
            enemies.push(e?);
        }
    }

    // 5. Render id -> English name (+ the resolved hint texts).
    let mut out_buf: Vec<u8> = Vec::new();
    writeln!(out_buf, "{}\t{}\t{}\t{}\t{}",
        "EnemyID", "Spawnable", "EnglishName", "KilledKnowledgeAction", "KnowledgeTipTexts")?;
    for e in &enemies {
        let name = dict.get(&e.id).map(clean).unwrap_or_default();
        let killed = e.killed_action.clone().unwrap_or_default();
        let mut tips = String::new();
        for k in &e.tip_keys {
            let text = dict.get(k).map(clean).unwrap_or_default();
            tips.push_str(&format!("[{}]={}", k, text));
        }
        writeln!(out_buf, "{}\t{}\t{}\t{}\t{}", e.id, e.spawnable, name, killed, tips)?;
    }

    if let Some(out_path) = args.get(2) {
        std::fs::write(out_path, &out_buf).with_context(|| format!("write {out_path}"))?;
        eprintln!("wrote {} bytes to {}", out_buf.len(), out_path);
    } else {
        std::io::stdout().write_all(&out_buf)?;
    }
    Ok(())
}