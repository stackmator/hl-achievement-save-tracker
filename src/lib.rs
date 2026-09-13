use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;
use anyhow::Context;
use byteorder::{LittleEndian, ReadBytesExt};
use oozextract::Extractor;
use rusqlite::Connection;
use serde::Serialize;

pub const PFA_43_ID: &str = "PFA_43";
pub const PFA_43_NAME: &str = "Finishing Touches";
pub const PFA_43_REQUIRED: usize = 34; // from game DB AchievementCriteria.Occurances for PFA_43

/// The 34 Ancient-Magic-eligible enemy classes, derived from game data
/// (PhoenixGameData.sqlite `EnemyDefinition` + `AchievementDefinition.PFA_43`
/// OneOfEachInit seeding semantics) and validated against 15 real saves:
/// every save shows exactly `registered == Instances` with this roster
/// (Troll_Armored and GoblinChieftain are seeded/never credited and excluded).
pub const ENEMY_TYPES: &[EnemyType] = &[
    // Ashwinders (6)
    EnemyType { id: "DW_Extortionist_Grunt",   name: "Ashwinder",             category: "Ashwinders", candidate: false },
    EnemyType { id: "DW_Extortionist_Soldier", name: "Ashwinder Executioner", category: "Ashwinders", candidate: false },
    EnemyType { id: "DW_Extortionist_Mage",    name: "Ashwinder Duellist",    category: "Ashwinders", candidate: false },
    EnemyType { id: "DW_Extortionist_Sniper",  name: "Ashwinder Ranger",      category: "Ashwinders", candidate: false },
    EnemyType { id: "DW_Extortionist_Tank",    name: "Ashwinder Tank",        category: "Ashwinders", candidate: false },
    EnemyType { id: "DW_Extortionist_Captain", name: "Ashwinder Captain",     category: "Ashwinders", candidate: false },

    // Poachers (6)
    EnemyType { id: "DW_Poacher_Grunt",   name: "Poacher",             category: "Poachers", candidate: false },
    EnemyType { id: "DW_Poacher_Soldier", name: "Poacher Executioner", category: "Poachers", candidate: false },
    EnemyType { id: "DW_Poacher_Mage",    name: "Poacher Duellist",    category: "Poachers", candidate: false },
    EnemyType { id: "DW_Poacher_Sniper",  name: "Poacher Ranger",      category: "Poachers", candidate: false },
    EnemyType { id: "DW_Poacher_Tank",    name: "Poacher Tank",        category: "Poachers", candidate: false },
    EnemyType { id: "DW_Poacher_Captain", name: "Poacher Captain",     category: "Poachers", candidate: false },

    // Goblin Loyalists (4 - NO Chieftain, that is quest-boss tier & never credited)
    EnemyType { id: "GoblinAssassin", name: "Loyalist Assassin", category: "Goblin Loyalists", candidate: false },
    EnemyType { id: "GoblinMelee",    name: "Loyalist Warrior",  category: "Goblin Loyalists", candidate: false },
    EnemyType { id: "GoblinMage",     name: "Loyalist Sentinel", category: "Goblin Loyalists", candidate: false },
    EnemyType { id: "GoblinSniper",   name: "Loyalist Ranger",   category: "Goblin Loyalists", candidate: false },

    // Inferi (1)
    EnemyType { id: "Inferius", name: "Inferius", category: "Inferi", candidate: false },

    // Dugbogs (3)
    EnemyType { id: "Dugbog_Coast", name: "Coastal Dugbog",  category: "Dugbogs", candidate: false },
    EnemyType { id: "Dugbog_Lake",  name: "Lake Dugbog",     category: "Dugbogs", candidate: false },
    EnemyType { id: "Dugbog_Marsh", name: "Marsh Dugbog",    category: "Dugbogs", candidate: false },

    // Spiders (9)
    EnemyType { id: "SpiderWoodlouse",        name: "Thornback Scurriour", category: "Spiders", candidate: false },
    EnemyType { id: "SpiderWoodlouseSpitter", name: "Thornback Ambusher",  category: "Spiders", candidate: false },
    EnemyType { id: "SpiderWoodlouseTank",    name: "Thornback Matriarch", category: "Spiders", candidate: false },
    EnemyType { id: "SpiderWoodlouseSniper",  name: "Thornback Shooter",   category: "Spiders", candidate: false },
    EnemyType { id: "SpiderAccromantula",     name: "Acromantula",         category: "Spiders", candidate: false },
    EnemyType { id: "SpiderVenomous",         name: "Venomous Scurriour",  category: "Spiders", candidate: false },
    EnemyType { id: "SpiderVenomousSpitter",  name: "Venomous Ambusher",   category: "Spiders", candidate: false },
    EnemyType { id: "SpiderVenomousSniper",   name: "Venomous Shooter",    category: "Spiders", candidate: false },
    EnemyType { id: "SpiderVenomousTank",     name: "Venomous Matriarch",  category: "Spiders", candidate: false },

    // Trolls (3 - NO Armored, that is a OneOfEachInit seed, provably never counted)
    EnemyType { id: "Troll_Forest",  name: "Forest Troll",  category: "Trolls", candidate: false },
    EnemyType { id: "Troll_Mountain",name: "Mountain Troll",category: "Trolls", candidate: false },
    EnemyType { id: "Troll_River",   name: "River Troll",   category: "Trolls", candidate: false },

    // Mongrels (2)
    EnemyType { id: "Wolf",    name: "Mongrel",     category: "Mongrels", candidate: false },
    EnemyType { id: "DW_Wolf", name: "Dark Mongrel",category: "Mongrels", candidate: false },
];

#[derive(Debug, Serialize, Clone)]
pub struct EnemyType {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub candidate: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct EnemyStatus {
    pub id: String,
    pub name: String,
    pub category: String,
    pub candidate: bool,
    /// true if this class appears in the save's OneOfEach registered pool
    pub registered: bool,
    /// registered-regular classes should count; candidate classes may not
    pub completed: bool,
}

#[derive(Debug, Serialize)]
pub struct AchievementStatus {
    pub achievement_id: String,
    pub achievement_name: String,
    pub total_enemies: usize,
    pub completed_enemies: usize,
    /// classes in the whitelist NOT registered in the save
    pub missing_enemies: Vec<EnemyStatus>,
    /// whitelist classes that ARE registered in the save
    pub completed_enemies_list: Vec<EnemyStatus>,
    /// classes registered in the save but not in the whitelist (named/boss/one-offs)
    pub registered_not_counted: Vec<String>,
    /// classes registered that appear to count but the count disagrees (pool vs Instances)
    pub squeeze_indicator: Option<String>,
    pub progress_percent: f32,
}

fn find_raw_database_image(data: &[u8]) -> anyhow::Result<usize> {
    let needle = b"RawDatabaseImage";
    for i in 0..data.len().saturating_sub(needle.len()) {
        if &data[i..i + needle.len()] == needle && data[i + needle.len()] == 0 {
            return Ok(i);
        }
    }
    anyhow::bail!("RawDatabaseImage not found");
}

fn read_fstring(cursor: &mut Cursor<&[u8]>) -> anyhow::Result<String> {
    let len = cursor.read_i32::<LittleEndian>()?;
    if len <= 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; len as usize];
    cursor.read_exact(&mut buf)?;
    if buf.last() == Some(&0) {
        buf.pop();
    }
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn parse_field_at(data: &[u8], field_offset: usize) -> anyhow::Result<(u64, u64, u64)> {
    let mut cursor = Cursor::new(&data[field_offset..]);

    let mut name = String::new();
    loop {
        let b = cursor.read_u8()?;
        if b == 0 {
            break;
        }
        name.push(b as char);
    }

    let _field_type = read_fstring(&mut cursor)?;
    let field_size = cursor.read_i64::<LittleEndian>()? as u64;
    let _elem_type = read_fstring(&mut cursor)?;

    let compressed_start = field_offset + 65;

    if compressed_start + 48 > data.len() {
        anyhow::bail!("Not enough data for package header");
    }

    let pkg_tag = u64::from_le_bytes(data[compressed_start..compressed_start + 8].try_into()?);
    if pkg_tag != 0x9E2A83C1 {
        anyhow::bail!("Invalid package tag: 0x{:X}", pkg_tag);
    }

    let chunk_comp = u64::from_le_bytes(data[compressed_start + 32..compressed_start + 40].try_into()?);
    let chunk_uncomp = u64::from_le_bytes(data[compressed_start + 40..compressed_start + 48].try_into()?);

    Ok((field_size, chunk_comp, chunk_uncomp))
}

fn find_all_packages(
    data: &[u8],
    compressed_start: usize,
) -> anyhow::Result<Vec<(usize, u64, u64)>> {
    let compressed_data = &data[compressed_start..];
    let mut packages = Vec::new();

    for i in 0..compressed_data.len().saturating_sub(48) {
        if &compressed_data[i..i + 8] == [0xC1, 0x83, 0x2A, 0x9E, 0x00, 0x00, 0x00, 0x00] {
            if i + 48 <= compressed_data.len() {
                let chunk0_comp = u64::from_le_bytes(compressed_data[i + 32..i + 40].try_into()?);
                let chunk0_uncomp = u64::from_le_bytes(compressed_data[i + 40..i + 48].try_into()?);
                if chunk0_uncomp == 131072 {
                    packages.push((i, chunk0_comp, chunk0_uncomp));
                }
            }
        }
    }

    Ok(packages)
}

fn decompress_chunk(extractor: &mut Extractor, oodle_data: &[u8], expected_size: usize) -> anyhow::Result<Vec<u8>> {
    let mut decompressed = vec![0u8; expected_size];
    extractor
        .read_from_slice(oodle_data, &mut decompressed)
        .context("Oodle decompression failed")?;
    Ok(decompressed)
}

pub fn extract_raw_database(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let field_offset = find_raw_database_image(data)?;
    let (field_size, _, _) = parse_field_at(data, field_offset)?;

    let compressed_start = field_offset + 65;
    let packages = find_all_packages(data, compressed_start)?;
    if packages.is_empty() {
        anyhow::bail!("No database packages found");
    }

    let mut extractor = Extractor::new();
    let mut full_db = Vec::new();

    for (pkg_idx, (pkg_offset, chunk_comp, chunk_uncomp)) in packages.iter().enumerate() {
        let abs_pkg_start = compressed_start + pkg_offset;
        let abs_oodle_start = abs_pkg_start + 48;
        let abs_oodle_end = abs_oodle_start + *chunk_comp as usize;

        if abs_oodle_end > data.len() {
            anyhow::bail!("Package {} extends past end of file", pkg_idx);
        }

        let oodle_data = &data[abs_oodle_start..abs_oodle_end];
        let decompressed = decompress_chunk(&mut extractor, oodle_data, *chunk_uncomp as usize)?;

        // Only first package has 8-byte prefix
        let chunk_data = if pkg_idx == 0 && decompressed.len() > 8 {
            &decompressed[8..]
        } else {
            &decompressed[..]
        };
        full_db.extend_from_slice(chunk_data);
    }

    let _ = field_size;
    Ok(full_db)
}

pub fn fix_sqlite_header(db: &mut [u8]) -> anyhow::Result<()> {
    if db.len() < 100 {
        anyhow::bail!("Database too small");
    }

    // Check if header is at offset 8
    if &db[0..16] != b"SQLite format 3\0" {
        if db.len() > 8 && &db[8..24] == b"SQLite format 3\0" {
            // Discard first 8 bytes
            let mut fixed = db[8..].to_vec();
            fixed.resize(131072, 0);

            let page_size = u16::from_be_bytes([fixed[16], fixed[17]]);
            let actual_pages = fixed.len() / page_size as usize;
            let pages_be = (actual_pages as u32).to_be_bytes();
            fixed[28] = pages_be[0];
            fixed[29] = pages_be[1];
            fixed[30] = pages_be[2];
            fixed[31] = pages_be[3];

            db.copy_from_slice(&fixed);
            return Ok(());
        }
    }

    // Fix page count for valid header
    let page_size = u16::from_be_bytes([db[16], db[17]]);
    let actual_pages = db.len() / page_size as usize;
    let pages_be = (actual_pages as u32).to_be_bytes();
    db[28] = pages_be[0];
    db[29] = pages_be[1];
    db[30] = pages_be[2];
    db[31] = pages_be[3];

    Ok(())
}

pub fn load_status(conn: &Connection) -> anyhow::Result<AchievementStatus> {
    let mut registered_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut instances: i64 = 0;

    let mut stmt = conn.prepare("SELECT Instances, OneOfEach FROM AchievementDynamic WHERE AchievementID = ?1")?;
    stmt.query_row([PFA_43_ID], |row| {
        instances = row.get(0)?;
        let one_of_each: String = row.get(1)?;
        for part in one_of_each.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                registered_set.insert(trimmed.to_string());
            }
        }
        Ok(())
    })?;

    let mut all_enemies = Vec::new();
    let mut missing_enemies = Vec::new();
    let mut completed_enemies_list = Vec::new();
    let mut registered_whitelist: Vec<String> = Vec::new();
    let mut registered_not_counted: Vec<String> = Vec::new();

    for enemy in ENEMY_TYPES {
        let registered = registered_set.contains(enemy.id);
        if registered {
            registered_whitelist.push(enemy.id.to_string());
        }

        let status = EnemyStatus {
            id: enemy.id.to_string(),
            name: enemy.name.to_string(),
            category: enemy.category.to_string(),
            candidate: enemy.candidate,
            registered,
            completed: registered,
        };

        if registered {
            completed_enemies_list.push(status.clone());
        } else {
            missing_enemies.push(status.clone());
        }
        all_enemies.push(status);
    }

    for id in &registered_set {
        if !all_enemies.iter().any(|e| e.id == *id) {
            registered_not_counted.push(id.clone());
        }
    }
    registered_not_counted.sort();

    let progress = if ENEMY_TYPES.is_empty() {
        0.0
    } else {
        (instances as f32 / PFA_43_REQUIRED as f32) * 100.0
    };

    let squeeze_indicator = if completed_enemies_list.len() > instances as usize {
        Some(format!(
            "{} whitelist classes are registered in the save pool, but the save only counts {} toward the trophy (suspect: one registered class is not trophy-eligible)."
            , completed_enemies_list.len(), instances))
    } else {
        None
    };

    Ok(AchievementStatus {
        achievement_id: PFA_43_ID.to_string(),
        achievement_name: PFA_43_NAME.to_string(),
        total_enemies: PFA_43_REQUIRED,
        completed_enemies: instances as usize,
        missing_enemies,
        completed_enemies_list,
        registered_not_counted,
        squeeze_indicator,
        progress_percent: progress,
    })
}

/// Full pipeline: read a .sav, extract the embedded DB, repair its header,
/// and query achievement progress.
pub fn analyze_save(path: &Path) -> anyhow::Result<AchievementStatus> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GVAS") {
        anyhow::bail!("Not a GVAS file");
    }

    let mut db_data = extract_raw_database(&data)?;
    fix_sqlite_header(&mut db_data)?;

    let temp_path = std::env::temp_dir().join("hl_save_tracker_test.db");
    fs::write(&temp_path, &db_data)?;
    let conn = Connection::open(&temp_path)?;

    load_status(&conn)
}

/// Same as [`analyze_save`] but with a caller-chosen temp db path (used by tests).
pub fn analyze_save_with_db<S: AsRef<Path>>(path: &Path, db_path: S) -> anyhow::Result<AchievementStatus> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GVAS") {
        anyhow::bail!("Not a GVAS file");
    }

    let mut db_data = extract_raw_database(&data)?;
    fix_sqlite_header(&mut db_data)?;

    let db_path = db_path.as_ref();
    fs::write(db_path, &db_data)?;
    let conn = Connection::open(db_path)?;

    load_status(&conn)
}