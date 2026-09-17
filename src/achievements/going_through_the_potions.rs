use rusqlite::Connection;
use serde::Serialize;

pub const PFA_27_ID: &str = "PFA_27";
pub const PFA_27_NAME: &str = "Going Through the Potions";
pub const PFA_27_REQUIRED: usize = 6; // brewable potions

/// The 6 brewable potions, using the IDs the game records in the PFA_27
/// OneOfEach pool. The pool/AchievementDynamic entries use runtime recipe
/// IDs that differ from the recipe names (e.g. the Focus Potion is recorded
/// as "AMFillPotion" because it refills Ancient Magic, the Wiggenweld Potion
/// as "WoundCleaning", and Thunderbrew as "AutoDamagePotion").
pub const POTION_TYPES: &[PotionType] = &[
    PotionType {
        id: "AMFillPotion",
        name: "Focus Potion",
    },
    PotionType {
        id: "AutoDamagePotion",
        name: "Thunderbrew",
    },
    PotionType {
        id: "Edurus",
        name: "Edurus Potion",
    },
    PotionType {
        id: "InvisibilityPotion",
        name: "Invisibility Potion",
    },
    PotionType {
        id: "Maxima",
        name: "Maxima Potion",
    },
    PotionType {
        id: "WoundCleaning",
        name: "Wiggenweld Potion",
    },
];

#[derive(Debug, Serialize, Clone)]
pub struct PotionType {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct PotionStatus {
    pub id: String,
    pub name: String,
    /// true if this potion appears in the save's OneOfEach registered pool
    pub brewed: bool,
}

#[derive(Debug, Serialize)]
pub struct PotionAchievementStatus {
    pub achievement_id: String,
    pub achievement_name: String,
    pub total_potions: usize,
    pub brewed_potions: usize,
    /// false when the save has no PFA_27 tracking data (brewing not started)
    pub tracked: bool,
    /// potions NOT yet brewed (from the save's pool)
    pub missing_potions: Vec<PotionStatus>,
    /// potions that have been brewed
    pub brewed_potions_list: Vec<PotionStatus>,
    /// pool entries that are not in the 6-potion roster (unexpected)
    pub pool_not_whitelist: Vec<String>,
    /// pool size vs Instances discrepancy warning
    pub squeeze_indicator: Option<String>,
    pub progress_percent: f32,
}

pub fn load_potion_status(conn: &Connection) -> anyhow::Result<PotionAchievementStatus> {
    let pool = match crate::achievements::load_pool(conn, PFA_27_ID)? {
        Some(pool) => pool,
        None => {
            let missing_potions: Vec<PotionStatus> = POTION_TYPES
                .iter()
                .map(|p| PotionStatus {
                    id: p.id.to_string(),
                    name: p.name.to_string(),
                    brewed: false,
                })
                .collect();
            return Ok(PotionAchievementStatus {
                achievement_id: PFA_27_ID.to_string(),
                achievement_name: PFA_27_NAME.to_string(),
                total_potions: PFA_27_REQUIRED,
                brewed_potions: 0,
                tracked: false,
                missing_potions,
                brewed_potions_list: Vec::new(),
                pool_not_whitelist: Vec::new(),
                squeeze_indicator: None,
                progress_percent: 0.0,
            });
        }
    };
    let instances = pool.instances;
    let brewed_set = &pool.registered;

    let mut all_potions = Vec::new();
    let mut missing_potions = Vec::new();
    let mut brewed_potions_list = Vec::new();
    let mut pool_not_whitelist: Vec<String> = Vec::new();

    for potion in POTION_TYPES {
        let brewed = brewed_set.contains(potion.id);
        let status = PotionStatus {
            id: potion.id.to_string(),
            name: potion.name.to_string(),
            brewed,
        };
        if brewed {
            brewed_potions_list.push(status.clone());
        } else {
            missing_potions.push(status.clone());
        }
        all_potions.push(status);
    }

    for id in brewed_set {
        if !all_potions.iter().any(|p| p.id == *id) {
            pool_not_whitelist.push(id.clone());
        }
    }
    pool_not_whitelist.sort();

    let progress = (instances as f32 / PFA_27_REQUIRED as f32) * 100.0;

    let squeeze_indicator = if brewed_potions_list.len() > instances as usize {
        Some(format!(
            "{} potions are registered in the save pool, but the save only counts {} toward the trophy (suspect: one registered potion is not trophy-eligible).",
            brewed_potions_list.len(), instances))
    } else {
        None
    };

    Ok(PotionAchievementStatus {
        achievement_id: PFA_27_ID.to_string(),
        achievement_name: PFA_27_NAME.to_string(),
        total_potions: PFA_27_REQUIRED,
        brewed_potions: instances as usize,
        tracked: true,
        missing_potions,
        brewed_potions_list,
        pool_not_whitelist,
        squeeze_indicator,
        progress_percent: progress,
    })
}
