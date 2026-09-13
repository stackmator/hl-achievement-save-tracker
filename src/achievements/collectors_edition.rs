use rusqlite::Connection;
use serde::Serialize;

pub const COLLECTORS_EDITION_ID: &str = "Collection";
pub const COLLECTORS_EDITION_NAME: &str = "Collector's Edition";

/// The 10 collection categories shown in the game's Collections menu, in the
/// order the CollectionDynamic bucket returns them. "WandStyle" is displayed
/// as "Wand Handles" (the in-game name).
pub const COLLECTION_CATEGORIES: &[CollectionCategory] = &[
    CollectionCategory {
        id: "Beasts",
        name: "Beasts",
    },
    CollectionCategory {
        id: "Brooms",
        name: "Brooms",
    },
    CollectionCategory {
        id: "Conjurations",
        name: "Conjurations",
    },
    CollectionCategory {
        id: "Enemies",
        name: "Enemies",
    },
    CollectionCategory {
        id: "Exploration",
        name: "Exploration",
    },
    CollectionCategory {
        id: "Gear",
        name: "Gear",
    },
    CollectionCategory {
        id: "Potions",
        name: "Potions",
    },
    CollectionCategory {
        id: "Seeds",
        name: "Seeds",
    },
    CollectionCategory {
        id: "Traits",
        name: "Traits",
    },
    CollectionCategory {
        id: "WandStyle",
        name: "Wand Handles",
    },
];

#[derive(Debug, Serialize, Clone)]
pub struct CollectionCategory {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct CollectionCategoryStatus {
    pub id: String,
    pub name: String,
    /// distinct items ever recorded as Obtained
    pub obtained: usize,
    /// distinct items present in this save's collection ledger
    pub total: usize,
}

/// Progress toward "Collector's Edition" (obtain every item in all
/// collections). Unlike the other achievements there is no AchievementDynamic
/// counter/pool: the save's `CollectionDynamic` table is an event ledger that
/// seeds one row per roster item (`Unknown`) and appends an `Obtained` row for
/// every pickup (potions can accumulate dozens of rows). Progress is therefore
/// "distinct ItemIDs that have at least one Obtained row" per category. The
/// denominator is the save's own ledger roster, so builds with DLC-era
/// additions (e.g. Gear 97 vs 103) stay internally consistent.
#[derive(Debug, Serialize)]
pub struct CollectorsEditionStatus {
    pub achievement_id: String,
    pub achievement_name: String,
    pub categories: Vec<CollectionCategoryStatus>,
    pub total_collected: usize,
    pub total_items: usize,
    /// false when the save has no CollectionDynamic tracking data
    pub tracked: bool,
    pub complete: bool,
    pub progress_percent: f32,
}

pub fn load_collectors_status(conn: &Connection) -> anyhow::Result<CollectorsEditionStatus> {
    let tracked = conn
        .query_row(
            "SELECT COUNT(DISTINCT ItemID) FROM CollectionDynamic",
            [],
            |r| r.get::<_, i64>(0),
        )?
        > 0;

    let mut categories = Vec::new();
    let mut total_collected = 0usize;
    let mut total_items = 0usize;

    if tracked {
        let mut total_stmt = conn.prepare(
            "SELECT COUNT(DISTINCT ItemID) FROM CollectionDynamic WHERE CategoryID = ?1",
        )?;
        let mut got_stmt = conn.prepare(
            "SELECT COUNT(DISTINCT ItemID) FROM CollectionDynamic WHERE CategoryID = ?1 AND ItemState = 'Obtained'",
        )?;
        for cat in COLLECTION_CATEGORIES {
            let total = total_stmt.query_row([cat.id], |r| r.get::<_, i64>(0))?.max(0) as usize;
            let obtained = got_stmt.query_row([cat.id], |r| r.get::<_, i64>(0))?.max(0) as usize;
            categories.push(CollectionCategoryStatus {
                id: cat.id.to_string(),
                name: cat.name.to_string(),
                obtained,
                total,
            });
            total_collected += obtained;
            total_items += total;
        }
    }

    let complete = tracked && total_collected >= total_items;
    let progress_percent = if total_items == 0 {
        0.0
    } else {
        (total_collected as f32 / total_items as f32) * 100.0
    };

    Ok(CollectorsEditionStatus {
        achievement_id: COLLECTORS_EDITION_ID.to_string(),
        achievement_name: COLLECTORS_EDITION_NAME.to_string(),
        categories,
        total_collected,
        total_items,
        tracked,
        complete,
        progress_percent,
    })
}