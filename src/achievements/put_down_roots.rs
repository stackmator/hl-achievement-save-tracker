use rusqlite::Connection;
use serde::Serialize;

pub const PFA_28_ID: &str = "PFA_28";
pub const PFA_28_NAME: &str = "Put Down Roots";
pub const PFA_28_REQUIRED: usize = 8; // growable plants (PhoenixGameData.sqlite PlantDefinition)

/// The 8 growable plant types, using the IDs the game records in the PFA_28
/// OneOfEach pool. Six IDs are confirmed 1:1 from real saves; the pool recorder
/// spells Shrivelfig as "ShrivelFig" (a runtime string, differs from the
/// PlantDefinition.PlantID casing), so that form is used here. Knotgrass and
/// VenomousTentacula have not been grown in any tracked save yet and their pool
/// IDs are inferred from PlantDefinition.
pub const PLANT_TYPES: &[PlantType] = &[
    PlantType {
        id: "ChompingCabbage_Plant",
        name: "Chinese Chomping Cabbage",
    },
    PlantType {
        id: "Dittany",
        name: "Dittany",
    },
    PlantType {
        id: "Fluxweed",
        name: "Fluxweed",
    },
    PlantType {
        id: "Knotgrass",
        name: "Knotgrass",
    },
    PlantType {
        id: "Mallowsweet",
        name: "Mallowsweet",
    },
    PlantType {
        id: "Mandrake",
        name: "Mandrake",
    },
    PlantType {
        id: "ShrivelFig",
        name: "Shrivelfig",
    },
    PlantType {
        id: "VenomousTentacula",
        name: "Venomous Tentacula",
    },
];

#[derive(Debug, Serialize, Clone)]
pub struct PlantType {
    pub id: &'static str,
    pub name: &'static str,
}

#[derive(Debug, Serialize, Clone)]
pub struct PlantStatus {
    pub id: String,
    pub name: String,
    /// true if this plant appears in the save's OneOfEach registered pool
    pub grown: bool,
}

#[derive(Debug, Serialize)]
pub struct PlantAchievementStatus {
    pub achievement_id: String,
    pub achievement_name: String,
    pub total_plants: usize,
    pub grown_plants: usize,
    /// false when the save has no PFA_28 tracking data (Room of Requirement not unlocked)
    pub tracked: bool,
    /// plants NOT yet grown (from the save's pool)
    pub missing_plants: Vec<PlantStatus>,
    /// plants that have been grown
    pub grown_plants_list: Vec<PlantStatus>,
    /// pool entries that are not in the 8-plant roster (unexpected)
    pub pool_not_whitelist: Vec<String>,
    /// pool size vs Instances discrepancy warning
    pub squeeze_indicator: Option<String>,
    pub progress_percent: f32,
}

pub fn load_plant_status(conn: &Connection) -> anyhow::Result<PlantAchievementStatus> {
    let pool = match crate::achievements::load_pool(conn, PFA_28_ID)? {
        Some(pool) => pool,
        None => {
            let missing_plants: Vec<PlantStatus> = PLANT_TYPES
                .iter()
                .map(|p| PlantStatus {
                    id: p.id.to_string(),
                    name: p.name.to_string(),
                    grown: false,
                })
                .collect();
            return Ok(PlantAchievementStatus {
                achievement_id: PFA_28_ID.to_string(),
                achievement_name: PFA_28_NAME.to_string(),
                total_plants: PFA_28_REQUIRED,
                grown_plants: 0,
                tracked: false,
                missing_plants,
                grown_plants_list: Vec::new(),
                pool_not_whitelist: Vec::new(),
                squeeze_indicator: None,
                progress_percent: 0.0,
            });
        }
    };
    let instances = pool.instances;
    let grown_set = &pool.registered;

    let mut all_plants = Vec::new();
    let mut missing_plants = Vec::new();
    let mut grown_plants_list = Vec::new();
    let mut pool_not_whitelist: Vec<String> = Vec::new();

    for plant in PLANT_TYPES {
        let grown = grown_set.contains(plant.id);
        let status = PlantStatus {
            id: plant.id.to_string(),
            name: plant.name.to_string(),
            grown,
        };
        if grown {
            grown_plants_list.push(status.clone());
        } else {
            missing_plants.push(status.clone());
        }
        all_plants.push(status);
    }

    for id in grown_set {
        if !all_plants.iter().any(|p| p.id == *id) {
            pool_not_whitelist.push(id.clone());
        }
    }
    pool_not_whitelist.sort();

    let progress = (instances as f32 / PFA_28_REQUIRED as f32) * 100.0;

    let squeeze_indicator = if grown_plants_list.len() > instances as usize {
        Some(format!(
            "{} plants are registered in the save pool, but the save only counts {} toward the trophy (suspect: one registered plant is not trophy-eligible).",
            grown_plants_list.len(), instances))
    } else {
        None
    };

    Ok(PlantAchievementStatus {
        achievement_id: PFA_28_ID.to_string(),
        achievement_name: PFA_28_NAME.to_string(),
        total_plants: PFA_28_REQUIRED,
        grown_plants: instances as usize,
        tracked: true,
        missing_plants,
        grown_plants_list,
        pool_not_whitelist,
        squeeze_indicator,
        progress_percent: progress,
    })
}
