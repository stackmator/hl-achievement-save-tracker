use clap::Parser;
use hl_save_tracker::{
    analyze_all, AchievementStatus, BeastAchievementStatus, CollectorsEditionStatus,
    MerlinAchievementStatus, PlantAchievementStatus, PotionAchievementStatus,
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

    /// Which achievements to report: both, enemies, beasts, plants, potions, merlin trials, or collections
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
    Collectors,
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
            writeln!(
                w,
                "(8 growable plants; pool recorder uses 'ShrivelFig' casing as registered)."
            )?;
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
            writeln!(
                w,
                "id,name,bred,adult_males,adult_females,unknown_gender,pair_status"
            )?;
            for b in status
                .bred_beasts_list
                .iter()
                .chain(status.missing_beasts.iter())
            {
                match &b.owned {
                    Some(owned) => writeln!(
                        w,
                        "{},{},{},{},{},{},{}",
                        b.id,
                        b.name,
                        b.bred,
                        owned.adult_males,
                        owned.adult_females,
                        owned.unknown_gender,
                        owned.pair_status()
                    )?,
                    None => writeln!(w, "{},{},{},,,,Ownership unavailable", b.id, b.name, b.bred)?,
                }
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
                match &b.owned {
                    Some(owned) => writeln!(
                        w,
                        "  {} {} - Adult males: {}, adult females: {}, unknown sex: {} - {}",
                        status_icon,
                        b.name,
                        owned.adult_males,
                        owned.adult_females,
                        owned.unknown_gender,
                        owned.pair_status()
                    )?,
                    None => writeln!(w, "  {} {} - Ownership unavailable", status_icon, b.name)?,
                }
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
            writeln!(
                w,
                "(12 breedable species; phoenix is excluded - not breedable)."
            )?;
            writeln!(
                w,
                "NOTE: Owned adult counts combine inventory + all four vivariums; offspring and classroom beasts are excluded."
            )?;
            writeln!(
                w,
                "A male/female pair must be together in a vivarium with a breeding pen to breed; pair ownership alone does not mean readiness."
            )?;
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

fn output_collectors_status<W: Write>(
    w: &mut W,
    status: &CollectorsEditionStatus,
    format: &OutputFormat,
) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => {
            writeln!(w, "{}", serde_json::to_string_pretty(status)?)?;
        }
        OutputFormat::Csv => {
            writeln!(w, "category,obtained,total")?;
            for c in &status.categories {
                writeln!(w, "{},{},{}", c.id, c.obtained, c.total)?;
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
                status.total_collected, status.total_items, status.progress_percent
            )?;
            writeln!(w)?;

            for c in &status.categories {
                let icon = if c.obtained >= c.total { "✅" } else { "❌" };
                let pct = if c.total == 0 {
                    0.0
                } else {
                    (c.obtained as f32 / c.total as f32) * 100.0
                };
                writeln!(
                    w,
                    "  {} {:<14} {}/{} ({:.1}%)",
                    icon, c.name, c.obtained, c.total, pct
                )?;
            }

            if status.complete {
                writeln!(w, "\nCollector's Edition COMPLETE!")?;
            } else {
                writeln!(
                    w,
                    "\nCollected: {}/{} items ({:.1}%)",
                    status.total_collected, status.total_items, status.progress_percent
                )?;
            }
            writeln!(
                w,
                "\nCounts are distinct items with an Obtained ledger entry (CollectionDynamic),"
            )?;
            writeln!(
                w,
                "and totals are the save's own collection rosters (DLC-era saves may show smaller totals)."
            )?;
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
            output_collectors_status(&mut out, &full.collectors, &format)?;
        }
    } else if args.report == Report::Enemies {
        output_status(&mut out, &full.enemies, &format, args.missing_only)?;
    } else if args.report == Report::Beasts {
        output_beast_status(&mut out, &full.beasts, &format)?;
    } else if args.report == Report::Plants {
        output_plant_status(&mut out, &full.plants, &format)?;
    } else if args.report == Report::Potions {
        output_potion_status(&mut out, &full.potions, &format)?;
    } else if args.report == Report::Merlin {
        output_merlin_status(&mut out, &full.merlin, &format)?;
    } else {
        output_collectors_status(&mut out, &full.collectors, &format)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn beast_report_fixture(ownership_available: bool) -> BeastAchievementStatus {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE AchievementDynamic (
                AchievementID TEXT, Instances INTEGER, OneOfEach TEXT
            );
            INSERT INTO AchievementDynamic VALUES ('PFA_26', 1, 'Diricawl');",
        )
        .unwrap();
        if ownership_available {
            conn.execute_batch(
                "CREATE TABLE NurturingCreatureDynamic (
                    TypeID TEXT, NurturingSpaceID TEXT, IsGenderMale INTEGER
                );",
            )
            .unwrap();
        }
        let mut status = hl_save_tracker::load_beast_status(&conn).unwrap();
        if ownership_available {
            for beast in status
                .bred_beasts_list
                .iter_mut()
                .chain(status.missing_beasts.iter_mut())
            {
                let owned = beast.owned.as_mut().unwrap();
                let (males, females, unknown) = match beast.id.as_str() {
                    "Diricawl" => (2, 3, 0),
                    "Fwooper" => (0, 2, 0),
                    "Graphorn" => (1, 0, 0),
                    "Unicorn" => (0, 0, 2),
                    _ => (0, 0, 0),
                };
                owned.adult_males = males;
                owned.adult_females = females;
                owned.unknown_gender = unknown;
            }
        }
        status
    }

    fn render_beasts(status: &BeastAchievementStatus, format: OutputFormat) -> String {
        let mut output = Vec::new();
        output_beast_status(&mut output, status, &format).unwrap();
        String::from_utf8(output).unwrap()
    }

    #[test]
    fn beast_table_displays_counts_and_pair_status() {
        let output = render_beasts(&beast_report_fixture(true), OutputFormat::Table);
        for expected in [
            "Diricawl - Adult males: 2, adult females: 3, unknown sex: 0 - Pair owned",
            "Fwooper - Adult males: 0, adult females: 2, unknown sex: 0 - Missing male",
            "Graphorn - Adult males: 1, adult females: 0, unknown sex: 0 - Missing female",
            "Niffler - Adult males: 0, adult females: 0, unknown sex: 0 - Missing male and female",
            "Unicorn - Adult males: 0, adult females: 0, unknown sex: 2 - Unknown (adult sex unavailable)",
            "Bred: 1/12 species",
            "counts combine inventory + all four vivariums",
            "offspring and classroom beasts are excluded",
            "pair must be together in a vivarium with a breeding pen",
            "pair ownership alone does not mean readiness",
        ] {
            assert!(output.contains(expected), "missing table text: {expected}");
        }
    }

    #[test]
    fn beast_table_displays_unavailable_ownership() {
        let status = beast_report_fixture(false);
        let output = render_beasts(&status, OutputFormat::Table);
        for beast in status
            .bred_beasts_list
            .iter()
            .chain(status.missing_beasts.iter())
        {
            assert!(output.contains(&format!("{} - Ownership unavailable", beast.name)));
        }
        assert!(!output.contains("Adult males:"));
        assert!(!output.contains("Missing male"));
    }

    #[test]
    fn beast_csv_displays_counts_and_pair_status() {
        let output = render_beasts(&beast_report_fixture(true), OutputFormat::Csv);
        assert_eq!(
            output.lines().next().unwrap(),
            "id,name,bred,adult_males,adult_females,unknown_gender,pair_status"
        );
        assert_eq!(output.lines().count(), 13);
        for expected in [
            "Diricawl,Diricawl,true,2,3,0,Pair owned",
            "Fwooper,Fwooper,false,0,2,0,Missing male",
            "Graphorn,Graphorn,false,1,0,0,Missing female",
            "Niffler,Niffler,false,0,0,0,Missing male and female",
            "Unicorn,Unicorn,false,0,0,2,Unknown (adult sex unavailable)",
        ] {
            assert!(
                output.lines().any(|line| line == expected),
                "missing CSV row: {expected}"
            );
        }
        assert!(output.lines().all(|line| line.split(',').count() == 7));
    }

    #[test]
    fn beast_csv_leaves_unavailable_counts_empty() {
        let output = render_beasts(&beast_report_fixture(false), OutputFormat::Csv);
        assert_eq!(output.lines().count(), 13);
        for line in output.lines().skip(1) {
            let cells: Vec<_> = line.split(',').collect();
            assert_eq!(cells.len(), 7);
            assert_eq!(&cells[3..6], &["", "", ""]);
            assert_eq!(cells[6], "Ownership unavailable");
        }
        assert!(output
            .lines()
            .any(|line| line == "Diricawl,Diricawl,true,,,,Ownership unavailable"));
    }

    #[test]
    fn beast_json_serializes_owned_counts_and_unknown_sex() {
        let output = render_beasts(&beast_report_fixture(true), OutputFormat::Json);
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(json["bred_beasts"], 1);
        let bred = &json["bred_beasts_list"][0];
        assert_eq!(bred["id"], "Diricawl");
        assert_eq!(bred["bred"], true);
        assert_eq!(bred["owned"]["adult_males"], 2);
        assert_eq!(bred["owned"]["adult_females"], 3);
        assert_eq!(bred["owned"]["unknown_gender"], 0);
        let missing = json["missing_beasts"].as_array().unwrap();
        let unicorn = missing
            .iter()
            .find(|beast| beast["id"] == "Unicorn")
            .unwrap();
        assert_eq!(unicorn["bred"], false);
        assert_eq!(unicorn["owned"]["adult_males"], 0);
        assert_eq!(unicorn["owned"]["adult_females"], 0);
        assert_eq!(unicorn["owned"]["unknown_gender"], 2);
        let niffler = missing
            .iter()
            .find(|beast| beast["id"] == "Niffler")
            .unwrap();
        assert_eq!(niffler["owned"]["adult_males"], 0);
        assert_eq!(niffler["owned"]["adult_females"], 0);
        assert_eq!(niffler["owned"]["unknown_gender"], 0);
    }

    #[test]
    fn beast_json_serializes_unavailable_ownership_as_null() {
        let output = render_beasts(&beast_report_fixture(false), OutputFormat::Json);
        let json: serde_json::Value = serde_json::from_str(&output).unwrap();
        let bred = json["bred_beasts_list"].as_array().unwrap();
        let missing = json["missing_beasts"].as_array().unwrap();
        assert_eq!(bred.len() + missing.len(), 12);
        for beast in bred.iter().chain(missing.iter()) {
            assert_eq!(beast.get("owned"), Some(&serde_json::Value::Null));
        }
    }
}
