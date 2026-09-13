use clap::Parser;
use hl_save_tracker::{
    analyze_all, AchievementStatus, BeastAchievementStatus, MerlinAchievementStatus,
    PlantAchievementStatus, PotionAchievementStatus,
};
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "hl-save-tracker",
    version,
    about = "Track Hogwarts Legacy achievements from save files"
)]
struct Args {
    /// Path to the save file (.sav)
    #[arg(short, long)]
    save: PathBuf,

    /// Output format
    #[arg(short, long, default_value = "table")]
    format: OutputFormat,

    /// Show only missing enemies
    #[arg(long)]
    missing_only: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Which achievements to report: both, enemies, beasts, plants, potions, or merlin trials
    #[arg(long, value_enum, default_value_t = Report::Both)]
    report: Report,

    /// Write report output to this file instead of stdout
    #[arg(short, long)]
    out: Option<PathBuf>,
}

#[derive(Debug, clap::ValueEnum, Clone, Default, PartialEq)]
enum Report {
    #[default]
    Both,
    Enemies,
    Beasts,
    Plants,
    Potions,
    Merlin,
}

#[derive(Debug, clap::ValueEnum, Clone, Default)]
enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}

fn output_status<W: Write>(
    w: &mut W,
    status: &AchievementStatus,
    format: &OutputFormat,
    missing_only: bool,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            if missing_only {
                writeln!(
                    w,
                    "{}",
                    serde_json::to_string_pretty(&status.missing_enemies)?
                )?;
            } else {
                writeln!(w, "{}", serde_json::to_string_pretty(status)?)?;
            }
        }
        OutputFormat::Csv => {
            writeln!(w, "id,name,category,candidate,registered,completed")?;
            let enemies: Vec<&hl_save_tracker::EnemyStatus> = if missing_only {
                status.missing_enemies.iter().collect()
            } else {
                status
                    .completed_enemies_list
                    .iter()
                    .chain(status.missing_enemies.iter())
                    .collect()
            };
            for e in enemies {
                writeln!(
                    w,
                    "{},{},{},{},{},{}",
                    e.id, e.name, e.category, e.candidate, e.registered, e.completed
                )?;
            }
        }
        OutputFormat::Table => {
            writeln!(
                w,
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            )?;
            writeln!(
                w,
                "Progress: {}/{} ({:.1}%)",
                status.completed_enemies, status.total_enemies, status.progress_percent
            )?;
            writeln!(w)?;

            if !status.tracked {
                writeln!(
                    w,
                    "NOTE: this save has no {} tracking data yet (fresh save). The full roster is shown as not started.",
                    status.achievement_id
                )?;
            }

            let enemies: Vec<&hl_save_tracker::EnemyStatus> = if missing_only {
                status.missing_enemies.iter().collect()
            } else {
                status
                    .completed_enemies_list
                    .iter()
                    .chain(status.missing_enemies.iter())
                    .collect()
            };

            let mut current_category = String::new();
            for e in enemies {
                if e.category != current_category {
                    current_category = e.category.to_string();
                    writeln!(w, "\n--- {} ---", current_category)?;
                }

                let status_icon = if e.registered { "✅" } else { "❌" };
                let candidate_str = if e.candidate { " (unverified)" } else { "" };
                writeln!(w, "  {} {}{}", status_icon, e.name, candidate_str)?;
            }

            writeln!(
                w,
                "\nTotal: {}/{} enemies completed ({:.1}%)",
                status.completed_enemies, status.total_enemies, status.progress_percent
            )?;

            if !status.registered_not_counted.is_empty() {
                writeln!(
                    w,
                    "\nRegistered in pool but NOT in the 34-class list (named/boss/one-offs): {}",
                    status.registered_not_counted.len()
                )?;
                writeln!(w, "  {}", status.registered_not_counted.join(", "))?;
            }
            if let Some(sq) = &status.squeeze_indicator {
                writeln!(w, "\nNOTE: {}", sq)?;
            }
            writeln!(
                w,
                "\nRoster derived from PhoenixGameData.sqlite (EnemyDefinition + PFA_43 OneOfEachInit)"
            )?;
            writeln!(
                w,
                "and validated over 15 saves: Instances == registered whitelist classes for all of them."
            )?;
        }
    }
    Ok(())
}

fn output_plant_status<W: Write>(
    w: &mut W,
    status: &PlantAchievementStatus,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::to_string_pretty(status)?)?;
        }
        OutputFormat::Csv => {
            writeln!(w, "id,name,grown")?;
            for p in status
                .grown_plants_list
                .iter()
                .chain(status.missing_plants.iter())
            {
                writeln!(w, "{},{},{}", p.id, p.name, p.grown)?;
            }
        }
        OutputFormat::Table => {
            writeln!(
                w,
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            )?;
            writeln!(
                w,
                "Progress: {}/{} ({:.1}%)",
                status.grown_plants, status.total_plants, status.progress_percent
            )?;
            writeln!(w)?;

            if !status.tracked {
                writeln!(
                    w,
                    "NOTE: this save has no {} tracking data yet (Room of Requirement not unlocked). The full roster is shown as not started.",
                    status.achievement_id
                )?;
            }

            for p in status
                .grown_plants_list
                .iter()
                .chain(status.missing_plants.iter())
            {
                let status_icon = if p.grown { "✅" } else { "❌" };
                writeln!(w, "  {} {}", status_icon, p.name)?;
            }

            writeln!(
                w,
                "\nGrown: {}/{} plants ({:.1}%)",
                status.grown_plants, status.total_plants, status.progress_percent
            )?;

            if !status.pool_not_whitelist.is_empty() {
                writeln!(
                    w,
                    "\nRegistered in pool but NOT in the 8-plant list: {}",
                    status.pool_not_whitelist.join(", ")
                )?;
            }
            if let Some(sq) = &status.squeeze_indicator {
                writeln!(w, "\nNOTE: {}", sq)?;
            }
            writeln!(
                w,
                "\nRoster derived from PhoenixGameData.sqlite (PlantDefinition + PFA_28 pool)"
            )?;
            writeln!(w, "(8 growable plants; pool recorder uses 'ShrivelFig' casing as registered).")?;
        }
    }
    Ok(())
}

fn output_beast_status<W: Write>(
    w: &mut W,
    status: &BeastAchievementStatus,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::to_string_pretty(status)?)?;
        }
        OutputFormat::Csv => {
            writeln!(w, "id,name,bred")?;
            for b in status
                .bred_beasts_list
                .iter()
                .chain(status.missing_beasts.iter())
            {
                writeln!(w, "{},{},{}", b.id, b.name, b.bred)?;
            }
        }
        OutputFormat::Table => {
            writeln!(
                w,
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            )?;
            writeln!(
                w,
                "Progress: {}/{} ({:.1}%)",
                status.bred_beasts, status.total_beasts, status.progress_percent
            )?;
            writeln!(w)?;

            if !status.tracked {
                writeln!(
                    w,
                    "NOTE: this save has no {} tracking data yet (breeding not started). The full roster is shown as not started.",
                    status.achievement_id
                )?;
            }

            for b in status
                .bred_beasts_list
                .iter()
                .chain(status.missing_beasts.iter())
            {
                let status_icon = if b.bred { "✅" } else { "❌" };
                writeln!(w, "  {} {}", status_icon, b.name)?;
            }

            writeln!(
                w,
                "\nBred: {}/{} species ({:.1}%)",
                status.bred_beasts, status.total_beasts, status.progress_percent
            )?;

            if !status.pool_not_whitelist.is_empty() {
                writeln!(
                    w,
                    "\nRegistered in pool but NOT in the 12-species list: {}",
                    status.pool_not_whitelist.join(", ")
                )?;
            }
            if let Some(sq) = &status.squeeze_indicator {
                writeln!(w, "\nNOTE: {}", sq)?;
            }
            writeln!(
                w,
                "\nRoster derived from the save's PFA_26 OneOfEach pool and NamedCreatureDefinition"
            )?;
            writeln!(w, "(12 breedable species; phoenix is excluded - not breedable).")?;
        }
    }
    Ok(())
}

fn output_potion_status<W: Write>(
    w: &mut W,
    status: &PotionAchievementStatus,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::to_string_pretty(status)?)?;
        }
        OutputFormat::Csv => {
            writeln!(w, "id,name,brewed")?;
            for p in status
                .brewed_potions_list
                .iter()
                .chain(status.missing_potions.iter())
            {
                writeln!(w, "{},{},{}", p.id, p.name, p.brewed)?;
            }
        }
        OutputFormat::Table => {
            writeln!(
                w,
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            )?;
            writeln!(
                w,
                "Progress: {}/{} ({:.1}%)",
                status.brewed_potions, status.total_potions, status.progress_percent
            )?;
            writeln!(w)?;

            if !status.tracked {
                writeln!(
                    w,
                    "NOTE: this save has no {} tracking data yet (brewing not started). The full roster is shown as not started.",
                    status.achievement_id
                )?;
            }

            for p in status
                .brewed_potions_list
                .iter()
                .chain(status.missing_potions.iter())
            {
                let status_icon = if p.brewed { "✅" } else { "❌" };
                writeln!(w, "  {} {}", status_icon, p.name)?;
            }

            writeln!(
                w,
                "\nBrewed: {}/{} potions ({:.1}%)",
                status.brewed_potions, status.total_potions, status.progress_percent
            )?;

            if !status.pool_not_whitelist.is_empty() {
                writeln!(
                    w,
                    "\nRegistered in pool but NOT in the 6-potion list: {}",
                    status.pool_not_whitelist.join(", ")
                )?;
            }
            if let Some(sq) = &status.squeeze_indicator {
                writeln!(w, "\nNOTE: {}", sq)?;
            }
            writeln!(
                w,
                "\nRoster derived from the save's PFA_27 OneOfEach pool (recipe IDs; e.g. Wiggenweld is recorded as WoundCleaning)."
            )?;
            writeln!(
                w,
                "(6 brewable potions; Focus is recorded as AMFillPotion and Thunderbrew as AutoDamagePotion)."
            )?;
        }
    }
    Ok(())
}

fn output_merlin_status<W: Write>(
    w: &mut W,
    status: &MerlinAchievementStatus,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::to_string_pretty(status)?)?;
        }
        OutputFormat::Csv => {
            writeln!(w, "id,name,completed,total")?;
            writeln!(
                w,
                "{},{},{},{}",
                status.achievement_id,
                status.achievement_name,
                status.completed_trials,
                status.total_trials
            )?;
        }
        OutputFormat::Table => {
            writeln!(
                w,
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            )?;
            writeln!(
                w,
                "Progress: {}/{} ({:.1}%)",
                status.completed_trials, status.total_trials, status.progress_percent
            )?;
            writeln!(w)?;

            if !status.tracked {
                writeln!(
                    w,
                    "NOTE: this save has no {} tracking data yet (no Merlin trials started).",
                    status.achievement_id
                )?;
            }

            writeln!(
                w,
                "\nCompleted: {}/{} Merlin Trials ({:.1}%)",
                status.completed_trials, status.total_trials, status.progress_percent
            )?;
            writeln!(
                w,
                "\nThe save records only the completion count (not individual trials), so the {} remaining trials are not listed individually.",
                status.total_trials.saturating_sub(status.completed_trials)
            )?;
            writeln!(w, "\nTotal of 95 Merlin Trials across the Highlands; count matches ACK_CompleteAll_MerlinTrials.")?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let mut out: Box<dyn Write> = match &args.out {
        Some(path) => Box::new(File::create(path)?),
        None => Box::new(io::stdout()),
    };

    writeln!(out, "Hogwarts Legacy Save Tracker")?;
    writeln!(out, "==============================")?;
    writeln!(out)?;

    writeln!(out, "Processing save file: {:?}", args.save)?;
    let full = analyze_all(&args.save)?;

    let format = if args.json {
        OutputFormat::Json
    } else {
        args.format
    };
    let json_format = args.json;

    if args.report == Report::Both {
        if json_format {
            writeln!(out, "{}", serde_json::to_string_pretty(&full)?)?;
        } else {
            output_status(&mut out, &full.enemies, &format, args.missing_only)?;
            output_beast_status(&mut out, &full.beasts, &format)?;
            output_plant_status(&mut out, &full.plants, &format)?;
            output_potion_status(&mut out, &full.potions, &format)?;
            output_merlin_status(&mut out, &full.merlin, &format)?;
        }
    } else if args.report == Report::Enemies {
        output_status(&mut out, &full.enemies, &format, args.missing_only)?;
    } else if args.report == Report::Beasts {
        output_beast_status(&mut out, &full.beasts, &format)?;
    } else if args.report == Report::Plants {
        output_plant_status(&mut out, &full.plants, &format)?;
    } else if args.report == Report::Potions {
        output_potion_status(&mut out, &full.potions, &format)?;
    } else {
        output_merlin_status(&mut out, &full.merlin, &format)?;
    }

    Ok(())
}