use rusqlite::Connection;
use serde::Serialize;

pub const PFA_26_ID: &str = "PFA_26";
pub const PFA_26_NAME: &str = "The Nature of the Beast";
pub const PFA_26_REQUIRED: usize = 12; // breedable species (phoenix is not breedable)

/// The 12 breedable beast species, matching the TypeIDs used in the game's
/// breeding/AchievementDynamic pool (see NamedCreatureDefinition and the
/// registered OneOfEach entries in real saves). Phoenix is excluded per the
/// achievement requirement ("does not include phoenixes").
pub const BEAST_TYPES: &[BeastType] = &[
    BeastType {
        id: "Diricawl",
        name: "Diricawl",
    },
    BeastType {
        id: "Fwooper",
        name: "Fwooper",
    },
    BeastType {
        id: "GiantPurpleToad",
        name: "Giant Purple Toad",
    },
    BeastType {
        id: "Graphorn",
        name: "Graphorn",
    },
    BeastType {
        id: "Hippogriff",
        name: "Hippogriff",
    },
    BeastType {
        id: "Jobberknoll",
        name: "Jobberknoll",
    },
    BeastType {
        id: "Kneazle",
        name: "Kneazle",
    },
    BeastType {
        id: "Mooncalf",
        name: "Mooncalf",
    },
    BeastType {
        id: "Niffler",
        name: "Niffler",
    },
    BeastType {
        id: "Puffskein",
        name: "Puffskein",
    },
    BeastType {
        id: "Thestral",
        name: "Thestral",
    },
    BeastType {
        id: "Unicorn",
        name: "Unicorn",
    },
];

#[derive(Debug, Serialize, Clone)]
pub struct BeastType {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct BeastStatus {
    pub id: String,
    pub name: String,
    /// true if this species appears in the save's OneOfEach registered pool
    pub bred: bool,
}

#[derive(Debug, Serialize)]
pub struct BeastAchievementStatus {
    pub achievement_id: String,
    pub achievement_name: String,
    pub total_beasts: usize,
    pub bred_beasts: usize,
    /// false when the save has no PFA_26 tracking data (breeding not started / not unlocked)
    pub tracked: bool,
    /// species NOT yet bred (from the save's pool)
    pub missing_beasts: Vec<BeastStatus>,
    /// species that have been bred
    pub bred_beasts_list: Vec<BeastStatus>,
    /// pool entries that are not in the 12-species roster (unexpected)
    pub pool_not_whitelist: Vec<String>,
    /// pool size vs Instances discrepancy warning
    pub squeeze_indicator: Option<String>,
    pub progress_percent: f32,
}

pub fn load_beast_status(conn: &Connection) -> anyhow::Result<BeastAchievementStatus> {
    let pool = match crate::achievements::load_pool(conn, PFA_26_ID)? {
        Some(pool) => pool,
        None => {
            let missing_beasts: Vec<BeastStatus> = BEAST_TYPES
                .iter()
                .map(|b| BeastStatus {
                    id: b.id.to_string(),
                    name: b.name.to_string(),
                    bred: false,
                })
                .collect();
            return Ok(BeastAchievementStatus {
                achievement_id: PFA_26_ID.to_string(),
                achievement_name: PFA_26_NAME.to_string(),
                total_beasts: PFA_26_REQUIRED,
                bred_beasts: 0,
                tracked: false,
                missing_beasts,
                bred_beasts_list: Vec::new(),
                pool_not_whitelist: Vec::new(),
                squeeze_indicator: None,
                progress_percent: 0.0,
            });
        }
    };
    let instances = pool.instances;
    let bred_set = &pool.registered;

    let mut all_beasts = Vec::new();
    let mut missing_beasts = Vec::new();
    let mut bred_beasts_list = Vec::new();
    let mut pool_not_whitelist: Vec<String> = Vec::new();

    for beast in BEAST_TYPES {
        let bred = bred_set.contains(beast.id);
        let status = BeastStatus {
            id: beast.id.to_string(),
            name: beast.name.to_string(),
            bred,
        };
        if bred {
            bred_beasts_list.push(status.clone());
        } else {
            missing_beasts.push(status.clone());
        }
        all_beasts.push(status);
    }

    for id in bred_set {
        if !all_beasts.iter().any(|b| b.id == *id) {
            pool_not_whitelist.push(id.clone());
        }
    }
    pool_not_whitelist.sort();

    let progress = (instances as f32 / PFA_26_REQUIRED as f32) * 100.0;

    let squeeze_indicator = if bred_beasts_list.len() > instances as usize {
        Some(format!(
            "{} species are registered in the save pool, but the save only counts {} toward the trophy (suspect: one registered beast is not breedable)."
            , bred_beasts_list.len(), instances))
    } else {
        None
    };

    Ok(BeastAchievementStatus {
        achievement_id: PFA_26_ID.to_string(),
        achievement_name: PFA_26_NAME.to_string(),
        total_beasts: PFA_26_REQUIRED,
        bred_beasts: instances as usize,
        tracked: true,
        missing_beasts,
        bred_beasts_list,
        pool_not_whitelist,
        squeeze_indicator,
        progress_percent: progress,
    })
}
