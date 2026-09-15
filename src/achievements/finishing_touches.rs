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
/// A 16th save caught one roster error: `DW_Poacher_Captain` ("Poacher
/// Duellist") has no in-game enemy behind it, while `AnimagusWolf` (the Poacher
/// animagus wolf form) IS credited by the save's Instances counter, so the
/// roster counts AnimagusWolf instead.
///
/// Display names were extracted empirically from the game files: enemy IDs
/// come from `EnemyDefinition`, and English names come from the MAIN-enUS
/// AVAFDICT localization dictionary (`enemy_names.exe <pak>`), matching each
/// ID's exact key (falling back to the `BP_Spider_Woodlouse_C` asset key for
/// the base Thornback).
pub const ENEMY_TYPES: &[EnemyType] = &[
    // Ashwinders (6)
    EnemyType {
        id: "DW_Extortionist_Grunt",
        name: "Ashwinder Scout",
        category: "Ashwinders",
        candidate: false,
    },
    EnemyType {
        id: "DW_Extortionist_Soldier",
        name: "Ashwinder Soldier",
        category: "Ashwinders",
        candidate: false,
    },
    EnemyType {
        id: "DW_Extortionist_Mage",
        name: "Ashwinder Assassin",
        category: "Ashwinders",
        candidate: false,
    },
    EnemyType {
        id: "DW_Extortionist_Sniper",
        name: "Ashwinder Ranger",
        category: "Ashwinders",
        candidate: false,
    },
    EnemyType {
        id: "DW_Extortionist_Tank",
        name: "Ashwinder Executioner",
        category: "Ashwinders",
        candidate: false,
    },
    EnemyType {
        id: "DW_Extortionist_Captain",
        name: "Ashwinder Duellist",
        category: "Ashwinders",
        candidate: false,
    },
    // Poachers (6)
    EnemyType {
        id: "DW_Poacher_Grunt",
        name: "Poacher Tracker",
        category: "Poachers",
        candidate: false,
    },
    EnemyType {
        id: "DW_Poacher_Soldier",
        name: "Poacher Stalker",
        category: "Poachers",
        candidate: false,
    },
    EnemyType {
        id: "DW_Poacher_Mage",
        name: "Poacher Animagus",
        category: "Poachers",
        candidate: false,
    },
    EnemyType {
        id: "DW_Poacher_Sniper",
        name: "Poacher Ranger",
        category: "Poachers",
        candidate: false,
    },
    EnemyType {
        id: "DW_Poacher_Tank",
        name: "Poacher Executioner",
        category: "Poachers",
        candidate: false,
    },
    EnemyType {
        id: "AnimagusWolf",
        name: "Wolf Animagus",
        category: "Poachers",
        candidate: false,
    },
    // Goblin Loyalists (4 - NO Chieftain, that is quest-boss tier & never credited)
    EnemyType {
        id: "GoblinAssassin",
        name: "Loyalist Assassin",
        category: "Goblin Loyalists",
        candidate: false,
    },
    EnemyType {
        id: "GoblinMelee",
        name: "Loyalist Warrior",
        category: "Goblin Loyalists",
        candidate: false,
    },
    EnemyType {
        id: "GoblinMage",
        name: "Loyalist Sentinel",
        category: "Goblin Loyalists",
        candidate: false,
    },
    EnemyType {
        id: "GoblinSniper",
        name: "Loyalist Ranger",
        category: "Goblin Loyalists",
        candidate: false,
    },
    // Inferi (1)
    EnemyType {
        id: "Inferius",
        name: "Inferius",
        category: "Inferi",
        candidate: false,
    },
    // Dugbogs (3)
    EnemyType {
        id: "Dugbog_Coast",
        name: "Stoneback Dugbog",
        category: "Dugbogs",
        candidate: false,
    },
    EnemyType {
        id: "Dugbog_Lake",
        name: "Great Spined Dugbog",
        category: "Dugbogs",
        candidate: false,
    },
    EnemyType {
        id: "Dugbog_Marsh",
        name: "Cottongrass Dugbog",
        category: "Dugbogs",
        candidate: false,
    },
    // Spiders (9)
    EnemyType {
        id: "SpiderWoodlouse",
        name: "Thornback Scurriour",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderWoodlouseSpitter",
        name: "Thornback Shooter",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderWoodlouseTank",
        name: "Thornback Matriarch",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderWoodlouseSniper",
        name: "Thornback Ambusher",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderAccromantula",
        name: "Acromantula",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderVenomous",
        name: "Venomous Scurriour",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderVenomousSpitter",
        name: "Venomous Shooter",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderVenomousSniper",
        name: "Venomous Ambusher",
        category: "Spiders",
        candidate: false,
    },
    EnemyType {
        id: "SpiderVenomousTank",
        name: "Venomous Matriarch",
        category: "Spiders",
        candidate: false,
    },
    // Trolls (3 - NO Armored, that is a OneOfEachInit seed, provably never counted)
    EnemyType {
        id: "Troll_Forest",
        name: "Forest Troll",
        category: "Trolls",
        candidate: false,
    },
    EnemyType {
        id: "Troll_Mountain",
        name: "Mountain Troll",
        category: "Trolls",
        candidate: false,
    },
    EnemyType {
        id: "Troll_River",
        name: "River Troll",
        category: "Trolls",
        candidate: false,
    },
    // Mongrels (2)
    EnemyType {
        id: "Wolf",
        name: "Dark Mongrel",
        category: "Mongrels",
        candidate: false,
    },
    EnemyType {
        id: "DW_Wolf",
        name: "Mongrel",
        category: "Mongrels",
        candidate: false,
    },
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
    /// false when the save has no PFA_43 tracking data (fresh save, tracking not started)
    pub tracked: bool,
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

pub fn load_status(conn: &Connection) -> anyhow::Result<AchievementStatus> {
    let pool = match crate::achievements::load_pool(conn, PFA_43_ID)? {
        Some(pool) => pool,
        None => {
            let missing_enemies: Vec<EnemyStatus> = ENEMY_TYPES
                .iter()
                .map(|e| EnemyStatus {
                    id: e.id.to_string(),
                    name: e.name.to_string(),
                    category: e.category.to_string(),
                    candidate: e.candidate,
                    registered: false,
                    completed: false,
                })
                .collect();
            return Ok(AchievementStatus {
                achievement_id: PFA_43_ID.to_string(),
                achievement_name: PFA_43_NAME.to_string(),
                total_enemies: PFA_43_REQUIRED,
                completed_enemies: 0,
                tracked: false,
                missing_enemies,
                completed_enemies_list: Vec::new(),
                registered_not_counted: Vec::new(),
                squeeze_indicator: None,
                progress_percent: 0.0,
            });
        }
    };
    let instances = pool.instances;
    let registered_set = &pool.registered;

    let mut all_enemies = Vec::new();
    let mut missing_enemies = Vec::new();
    let mut completed_enemies_list = Vec::new();
    let mut registered_not_counted: Vec<String> = Vec::new();

    for enemy in ENEMY_TYPES {
        let registered = registered_set.contains(enemy.id);

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

    for id in registered_set {
        if !all_enemies.iter().any(|e| e.id == *id) {
            registered_not_counted.push(id.clone());
        }
    }
    registered_not_counted.sort();

    let progress = (instances as f32 / PFA_43_REQUIRED as f32) * 100.0;

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
        tracked: true,
        missing_enemies,
        completed_enemies_list,
        registered_not_counted,
        squeeze_indicator,
        progress_percent: progress,
    })
}
