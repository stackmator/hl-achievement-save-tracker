use rusqlite::Connection;
use serde::Serialize;
use std::collections::HashMap;

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
    pub owned: Option<OwnedBeasts>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct OwnedBeasts {
    pub adult_males: usize,
    pub adult_females: usize,
    pub unknown_gender: usize,
}

impl OwnedBeasts {
    pub fn pair_status(&self) -> &'static str {
        match (self.adult_males > 0, self.adult_females > 0) {
            (true, true) => "Pair owned",
            _ if self.unknown_gender > 0 => "Unknown (adult sex unavailable)",
            (false, true) => "Missing male",
            (true, false) => "Missing female",
            (false, false) => "Missing male and female",
        }
    }
}

fn load_owned_beasts(conn: &Connection) -> anyhow::Result<Option<HashMap<String, OwnedBeasts>>> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'NurturingCreatureDynamic' COLLATE NOCASE)",
        [],
        |row| row.get(0),
    )?;
    if !exists {
        return Ok(None);
    }

    let mut owned: HashMap<String, OwnedBeasts> = BEAST_TYPES
        .iter()
        .map(|beast| (beast.id.to_string(), OwnedBeasts::default()))
        .collect();
    let mut stmt = conn.prepare(
        "SELECT TypeID, IsGenderMale FROM NurturingCreatureDynamic
         WHERE NurturingSpaceID COLLATE NOCASE IN (
             'Inventory', 'NV_Biome_Coastal', 'NV_Biome_Forest',
             'NV_Biome_Grassland', 'NV_Biome_Swamp'
         )",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
    })?;
    for row in rows {
        let (type_id, gender) = row?;
        if let Some(beast) = BEAST_TYPES
            .iter()
            .find(|beast| beast.id.eq_ignore_ascii_case(&type_id))
        {
            let counts = owned.get_mut(beast.id).unwrap();
            match gender {
                Some(1) => counts.adult_males += 1,
                Some(0) => counts.adult_females += 1,
                _ => counts.unknown_gender += 1,
            }
        }
    }
    Ok(Some(owned))
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
    let owned = load_owned_beasts(conn)?;
    let pool = match crate::achievements::load_pool(conn, PFA_26_ID)? {
        Some(pool) => pool,
        None => {
            let missing_beasts: Vec<BeastStatus> = BEAST_TYPES
                .iter()
                .map(|b| BeastStatus {
                    id: b.id.to_string(),
                    name: b.name.to_string(),
                    bred: false,
                    owned: owned.as_ref().and_then(|counts| counts.get(b.id)).cloned(),
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
            owned: owned
                .as_ref()
                .and_then(|counts| counts.get(beast.id))
                .cloned(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn database(with_creatures: bool) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE AchievementDynamic (AchievementID TEXT, Instances INTEGER, OneOfEach TEXT);",
        )
        .unwrap();
        if with_creatures {
            conn.execute_batch(
                "CREATE TABLE NurturingCreatureDynamic (
                    CreatureUID INTEGER PRIMARY KEY, TypeID TEXT NOT NULL,
                    NurturingSpaceID TEXT, IsGenderMale INTEGER,
                    BreedingGeneration INTEGER DEFAULT 0, IsMount INTEGER DEFAULT 0
                );",
            )
            .unwrap();
        }
        conn
    }

    fn counts<'a>(status: &'a BeastAchievementStatus, id: &str) -> &'a OwnedBeasts {
        status
            .missing_beasts
            .iter()
            .chain(status.bred_beasts_list.iter())
            .find(|beast| beast.id == id)
            .unwrap()
            .owned
            .as_ref()
            .unwrap()
    }

    #[test]
    fn counts_adults_across_inventory_and_all_vivariums() {
        let conn = database(true);
        for (location, male) in [
            ("Inventory", 1),
            ("NV_Biome_Coastal", 0),
            ("NV_Biome_Forest", 1),
            ("NV_Biome_Grassland", 0),
            ("NV_Biome_Swamp", 1),
        ] {
            conn.execute(
                "INSERT INTO NurturingCreatureDynamic (TypeID, NurturingSpaceID, IsGenderMale)
                 VALUES ('Niffler', ?1, ?2)",
                rusqlite::params![location, male],
            )
            .unwrap();
        }
        conn.execute_batch(
            "INSERT INTO NurturingCreatureDynamic (TypeID, NurturingSpaceID, IsGenderMale, IsMount)
             VALUES ('Hippogriff', 'Inventory', 1, 1), ('Hippogriff', 'NV_Biome_Coastal', 0, 0);",
        )
        .unwrap();
        let status = load_beast_status(&conn).unwrap();
        assert!(!status.tracked);
        assert_eq!(status.bred_beasts, 0);
        assert_eq!(counts(&status, "Niffler").adult_males, 3);
        assert_eq!(counts(&status, "Niffler").adult_females, 2);
        assert_eq!(counts(&status, "Niffler").pair_status(), "Pair owned");
        assert_eq!(counts(&status, "Hippogriff").pair_status(), "Pair owned");
    }

    #[test]
    fn excludes_offspring_unowned_locations_and_nonbreedable_types() {
        let conn = database(true);
        conn.execute_batch(
            "INSERT INTO NurturingCreatureDynamic (TypeID, NurturingSpaceID, IsGenderMale, BreedingGeneration)
             VALUES ('Fwooper', 'Inventory', 0, 0),
                    ('FwooperOffspring', 'Inventory', 1, 0),
                    ('FwooperOffspring', 'NV_Biome_Forest', 1, 1),
                    ('Fwooper', 'Beast_Class', 1, 0),
                    ('Fwooper', 'Beast_Class2', 1, 0),
                    ('Fwooper', 'Beast_Class3', 1, 0),
                    ('Fwooper', 'None', 1, 0),
                    ('Fwooper', NULL, 1, 0),
                    ('Fwooper', 'NV_Biome_Unknown', 1, 0),
                    ('Phoenix', 'Inventory', 1, 0),
                    ('UnknownBeast', 'Inventory', 1, 0);
             INSERT INTO AchievementDynamic VALUES ('PFA_26', 1, 'Fwooper,');",
        )
        .unwrap();
        let status = load_beast_status(&conn).unwrap();
        assert_eq!(status.bred_beasts, 1);
        assert_eq!(status.bred_beasts_list[0].id, "Fwooper");
        assert_eq!(status.missing_beasts.len(), 11);
        assert_eq!(counts(&status, "Fwooper").adult_males, 0);
        assert_eq!(counts(&status, "Fwooper").adult_females, 1);
        assert_eq!(counts(&status, "Fwooper").pair_status(), "Missing male");
    }

    #[test]
    fn handles_unknown_sex_and_case_insensitive_ids() {
        let conn = database(true);
        conn.execute_batch(
            "INSERT INTO NurturingCreatureDynamic (TypeID, NurturingSpaceID, IsGenderMale)
             VALUES ('unicorn', 'inventory', NULL), ('Unicorn', 'Inventory', 2),
                    ('Graphorn', 'Inventory', 1), ('Niffler', 'Inventory', 0);",
        )
        .unwrap();
        let status = load_beast_status(&conn).unwrap();
        let unicorn = counts(&status, "Unicorn");
        assert_eq!(unicorn.adult_males, 0);
        assert_eq!(unicorn.adult_females, 0);
        assert_eq!(unicorn.unknown_gender, 2);
        assert_eq!(unicorn.pair_status(), "Unknown (adult sex unavailable)");
        assert_eq!(counts(&status, "Graphorn").pair_status(), "Missing female");
        assert_eq!(counts(&status, "Niffler").pair_status(), "Missing male");
        assert_eq!(
            counts(&status, "Diricawl").pair_status(),
            "Missing male and female"
        );
        assert_eq!(
            OwnedBeasts {
                adult_males: 1,
                adult_females: 1,
                unknown_gender: 1
            }
            .pair_status(),
            "Pair owned"
        );
    }

    #[test]
    fn distinguishes_missing_table_from_empty_owned_roster() {
        let conn = database(false);
        let status = load_beast_status(&conn).unwrap();
        assert!(status
            .missing_beasts
            .iter()
            .all(|beast| beast.owned.is_none()));
        let conn = database(true);
        let status = load_beast_status(&conn).unwrap();
        assert!(status.missing_beasts.iter().all(|beast| {
            beast.owned.as_ref().unwrap().pair_status() == "Missing male and female"
        }));
    }

    #[test]
    fn propagates_unreadable_ownership_schema() {
        let conn = database(false);
        conn.execute_batch("CREATE TABLE NurturingCreatureDynamic (TypeID TEXT);")
            .unwrap();
        assert!(load_beast_status(&conn).is_err());
    }
}
