use rusqlite::{Connection, OptionalExtension};
use std::collections::HashSet;

pub mod finishing_touches;
pub mod nature_of_the_beast;
pub mod put_down_roots;

pub use finishing_touches::{
    load_status, AchievementStatus, EnemyStatus, EnemyType, ENEMY_TYPES, PFA_43_ID, PFA_43_NAME,
    PFA_43_REQUIRED,
};
pub use nature_of_the_beast::{
    load_beast_status, BeastAchievementStatus, BeastStatus, BeastType, BEAST_TYPES, PFA_26_ID,
    PFA_26_NAME, PFA_26_REQUIRED,
};
pub use put_down_roots::{
    load_plant_status, PlantAchievementStatus, PlantStatus, PlantType, PFA_28_ID, PFA_28_NAME,
    PFA_28_REQUIRED, PLANT_TYPES,
};

/// The tracking data all OneOfEach-type achievements share: the game records a
/// comma-joined pool of registered items plus an Instances counter, both in
/// `AchievementDynamic`. Returns `None` when the save has no row yet for the
/// achievement (tracking not started).
pub(crate) struct PoolData {
    pub instances: i64,
    pub registered: HashSet<String>,
}

pub(crate) fn load_pool(
    conn: &Connection,
    achievement_id: &str,
) -> anyhow::Result<Option<PoolData>> {
    let row: Option<(i64, String)> = {
        let mut stmt = conn.prepare(
            "SELECT Instances, OneOfEach FROM AchievementDynamic WHERE AchievementID = ?1",
        )?;
        stmt.query_row([achievement_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?
    };

    match row {
        Some((instances, one_of_each)) => {
            let mut registered = HashSet::new();
            for part in one_of_each.split(',') {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    registered.insert(trimmed.to_string());
                }
            }
            Ok(Some(PoolData {
                instances,
                registered,
            }))
        }
        None => Ok(None),
    }
}
