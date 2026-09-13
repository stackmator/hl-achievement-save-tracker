use rusqlite::Connection;
use serde::Serialize;

pub const PFA_37_ID: &str = "PFA_37";
pub const PFA_37_NAME: &str = "Merlin's Beard!";
pub const PFA_37_REQUIRED: usize = 95; // total Merlin Trials across the Highlands

/// Merlin's Beard is a plain counter, not a OneOfEach pool: the save's
/// `AchievementDynamic` row for PFA_37 stores the number of completed Merlin
/// Trials in `Instances` (empty `OneOfEach`). The count matches
/// ACK_CompleteAll_MerlinTrials exactly (checked across 15 real saves). The
/// total of 95 comes from the game world (no DLC has added trials), since the
/// save never stores the full roster.
#[derive(Debug, Serialize)]
pub struct MerlinAchievementStatus {
    pub achievement_id: String,
    pub achievement_name: String,
    pub total_trials: usize,
    pub completed_trials: usize,
    /// false when the save has no PFA_37 tracking data (no Merlin trials started)
    pub tracked: bool,
    pub progress_percent: f32,
}

pub fn load_merlin_status(conn: &Connection) -> anyhow::Result<MerlinAchievementStatus> {
    let completed = match crate::achievements::load_pool(conn, PFA_37_ID)? {
        Some(pool) => pool.instances.max(0) as usize,
        None => {
            return Ok(MerlinAchievementStatus {
                achievement_id: PFA_37_ID.to_string(),
                achievement_name: PFA_37_NAME.to_string(),
                total_trials: PFA_37_REQUIRED,
                completed_trials: 0,
                tracked: false,
                progress_percent: 0.0,
            });
        }
    };

    Ok(MerlinAchievementStatus {
        achievement_id: PFA_37_ID.to_string(),
        achievement_name: PFA_37_NAME.to_string(),
        total_trials: PFA_37_REQUIRED,
        completed_trials: completed,
        tracked: true,
        progress_percent: (completed as f32 / PFA_37_REQUIRED as f32) * 100.0,
    })
}