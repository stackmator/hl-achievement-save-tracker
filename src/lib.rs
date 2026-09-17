//! Save extraction + analysis pipeline for the Hogwarts Legacy save embedded
//! database, plus per-achievement progress trackers.
//!
//! Each achievement lives in `achievements/` (one module per achievement);
//! this root module owns the .sav -> SQLite extraction pipeline.

use anyhow::Context;
use byteorder::{LittleEndian, ReadBytesExt};
use oozextract::Extractor;
use rusqlite::Connection;
use std::fs;
use std::io::{Cursor, Read};
use std::path::Path;

pub mod achievements;
pub mod avafdict;
pub mod pak;

pub use achievements::collectors_edition::{
    load_collectors_status, CollectionCategory, CollectionCategoryStatus, CollectorsEditionStatus,
    COLLECTION_CATEGORIES, COLLECTORS_EDITION_ID, COLLECTORS_EDITION_NAME,
};
pub use achievements::finishing_touches::{
    load_status, AchievementStatus, EnemyStatus, EnemyType, ENEMY_TYPES, PFA_43_ID, PFA_43_NAME,
    PFA_43_REQUIRED,
};
pub use achievements::going_through_the_potions::{
    load_potion_status, PotionAchievementStatus, PotionStatus, PotionType, PFA_27_ID, PFA_27_NAME,
    PFA_27_REQUIRED, POTION_TYPES,
};
pub use achievements::merlins_beard::{
    load_merlin_status, MerlinAchievementStatus, PFA_37_ID, PFA_37_NAME, PFA_37_REQUIRED,
};
pub use achievements::nature_of_the_beast::{
    load_beast_status, BeastAchievementStatus, BeastStatus, BeastType, OwnedBeasts, BEAST_TYPES,
    PFA_26_ID, PFA_26_NAME, PFA_26_REQUIRED,
};
pub use achievements::put_down_roots::{
    load_plant_status, PlantAchievementStatus, PlantStatus, PlantType, PFA_28_ID, PFA_28_NAME,
    PFA_28_REQUIRED, PLANT_TYPES,
};

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

    let chunk_comp =
        u64::from_le_bytes(data[compressed_start + 32..compressed_start + 40].try_into()?);
    let chunk_uncomp =
        u64::from_le_bytes(data[compressed_start + 40..compressed_start + 48].try_into()?);

    Ok((field_size, chunk_comp, chunk_uncomp))
}

fn find_all_packages(
    data: &[u8],
    compressed_start: usize,
) -> anyhow::Result<Vec<(usize, u64, u64)>> {
    let compressed_data = &data[compressed_start..];
    let mut packages = Vec::new();

    for i in 0..compressed_data.len().saturating_sub(48) {
        if compressed_data[i..i + 8] == [0xC1, 0x83, 0x2A, 0x9E, 0x00, 0x00, 0x00, 0x00]
            && i + 48 <= compressed_data.len()
        {
            let chunk0_comp = u64::from_le_bytes(compressed_data[i + 32..i + 40].try_into()?);
            let chunk0_uncomp = u64::from_le_bytes(compressed_data[i + 40..i + 48].try_into()?);
            if chunk0_uncomp == 131072 {
                packages.push((i, chunk0_comp, chunk0_uncomp));
            }
        }
    }

    Ok(packages)
}

fn decompress_chunk(
    extractor: &mut Extractor,
    oodle_data: &[u8],
    expected_size: usize,
) -> anyhow::Result<Vec<u8>> {
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
    if &db[0..16] != b"SQLite format 3\0" && db.len() > 8 && &db[8..24] == b"SQLite format 3\0" {
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

/// Full pipeline: read a .sav, extract the embedded DB, repair its header,
/// and report Finishing Touches progress.
pub fn analyze_save(path: &Path) -> anyhow::Result<AchievementStatus> {
    let conn = open_save_db(path)?;
    achievements::finishing_touches::load_status(&conn)
}

/// Same as [`analyze_save`] but with a caller-chosen temp db path (used by tests).
pub fn analyze_save_with_db<S: AsRef<Path>>(
    path: &Path,
    db_path: S,
) -> anyhow::Result<AchievementStatus> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GVAS") {
        anyhow::bail!("Not a GVAS file");
    }

    let mut db_data = extract_raw_database(&data)?;
    fix_sqlite_header(&mut db_data)?;

    let db_path = db_path.as_ref();
    fs::write(db_path, &db_data)?;
    let conn = Connection::open(db_path)?;

    achievements::finishing_touches::load_status(&conn)
}

/// Progress across every supported achievement.
#[derive(Debug, serde::Serialize)]
pub struct GameStatus {
    pub enemies: AchievementStatus,
    pub beasts: BeastAchievementStatus,
    pub plants: PlantAchievementStatus,
    pub potions: PotionAchievementStatus,
    pub merlin: MerlinAchievementStatus,
    pub collectors: CollectorsEditionStatus,
}

/// Extracts the DB once and reports progress for all supported achievements.
pub fn analyze_all(path: &Path) -> anyhow::Result<GameStatus> {
    let conn = open_save_db(path)?;
    Ok(GameStatus {
        enemies: achievements::finishing_touches::load_status(&conn)?,
        beasts: achievements::nature_of_the_beast::load_beast_status(&conn)?,
        plants: achievements::put_down_roots::load_plant_status(&conn)?,
        potions: achievements::going_through_the_potions::load_potion_status(&conn)?,
        merlin: achievements::merlins_beard::load_merlin_status(&conn)?,
        collectors: achievements::collectors_edition::load_collectors_status(&conn)?,
    })
}

/// Like [`analyze_all`] but with a caller-chosen temp db path (used by tests).
pub fn analyze_all_with_db<S: AsRef<Path>>(path: &Path, db_path: S) -> anyhow::Result<GameStatus> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GVAS") {
        anyhow::bail!("Not a GVAS file");
    }

    let mut db_data = extract_raw_database(&data)?;
    fix_sqlite_header(&mut db_data)?;

    let db_path = db_path.as_ref();
    fs::write(db_path, &db_data)?;
    let conn = Connection::open(db_path)?;

    Ok(GameStatus {
        enemies: achievements::finishing_touches::load_status(&conn)?,
        beasts: achievements::nature_of_the_beast::load_beast_status(&conn)?,
        plants: achievements::put_down_roots::load_plant_status(&conn)?,
        potions: achievements::going_through_the_potions::load_potion_status(&conn)?,
        merlin: achievements::merlins_beard::load_merlin_status(&conn)?,
        collectors: achievements::collectors_edition::load_collectors_status(&conn)?,
    })
}

fn open_save_db(path: &Path) -> anyhow::Result<Connection> {
    let data = fs::read(path)?;
    if !data.starts_with(b"GVAS") {
        anyhow::bail!("Not a GVAS file");
    }

    let mut db_data = extract_raw_database(&data)?;
    fix_sqlite_header(&mut db_data)?;

    let temp_path = std::env::temp_dir().join("hl_save_tracker_test.db");
    fs::write(&temp_path, &db_data)?;
    Ok(Connection::open(&temp_path)?)
}
