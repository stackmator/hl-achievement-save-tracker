use clap::Parser;
use hl_save_tracker::{analyze_all, AchievementStatus, BeastAchievementStatus};
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

    /// Which achievements to report: both, enemies, or beasts
    #[arg(long, value_enum, default_value_t = Report::Both)]
    report: Report,
}

#[derive(Debug, clap::ValueEnum, Clone, Default, PartialEq)]
enum Report {
    #[default]
    Both,
    Enemies,
    Beasts,
}

#[derive(Debug, clap::ValueEnum, Clone, Default)]
enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}

fn output_status(status: &AchievementStatus, format: &OutputFormat, missing_only: bool) {
    match format {
        OutputFormat::Json => {
            if missing_only {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&status.missing_enemies).unwrap()
                );
            } else {
                println!("{}", serde_json::to_string_pretty(status).unwrap());
            }
        }
        OutputFormat::Csv => {
            println!("id,name,category,candidate,registered,completed");
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
                println!(
                    "{},{},{},{},{},{}",
                    e.id, e.name, e.category, e.candidate, e.registered, e.completed
                );
            }
        }
        OutputFormat::Table => {
            println!(
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            );
            println!(
                "Progress: {}/{} ({:.1}%)",
                status.completed_enemies, status.total_enemies, status.progress_percent
            );
            println!();

            if !status.tracked {
                println!("NOTE: this save has no {} tracking data yet (fresh save). The full roster is shown as not started.",
                    status.achievement_id);
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
                    println!("\n--- {} ---", current_category);
                }

                let status_icon = if e.registered { "✅" } else { "❌" };
                let candidate_str = if e.candidate { " (unverified)" } else { "" };
                println!("  {} {}{}", status_icon, e.name, candidate_str);
            }

            println!(
                "\nTotal: {}/{} enemies completed ({:.1}%)",
                status.completed_enemies, status.total_enemies, status.progress_percent
            );

            if !status.registered_not_counted.is_empty() {
                println!(
                    "\nRegistered in pool but NOT in the 34-class list (named/boss/one-offs): {}",
                    status.registered_not_counted.len()
                );
                println!("  {}", status.registered_not_counted.join(", "));
            }
            if let Some(sq) = &status.squeeze_indicator {
                println!("\nNOTE: {}", sq);
            }
            println!("\nRoster derived from PhoenixGameData.sqlite (EnemyDefinition + PFA_43 OneOfEachInit)");
            println!("and validated over 15 saves: Instances == registered whitelist classes for all of them.");
        }
    }
}

#[derive(Debug, serde::Serialize)]
struct FullReport {
    enemies: AchievementStatus,
    beasts: BeastAchievementStatus,
}

fn output_beast_status(status: &BeastAchievementStatus, format: &OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(status).unwrap());
        }
        OutputFormat::Csv => {
            println!("id,name,bred");
            for b in status
                .bred_beasts_list
                .iter()
                .chain(status.missing_beasts.iter())
            {
                println!("{},{},{}", b.id, b.name, b.bred);
            }
        }
        OutputFormat::Table => {
            println!(
                "\n=== {} ({}) ===",
                status.achievement_name, status.achievement_id
            );
            println!(
                "Progress: {}/{} ({:.1}%)",
                status.bred_beasts, status.total_beasts, status.progress_percent
            );
            println!();

            if !status.tracked {
                println!("NOTE: this save has no {} tracking data yet (breeding not started). The full roster is shown as not started.",
                    status.achievement_id);
            }

            for b in status
                .bred_beasts_list
                .iter()
                .chain(status.missing_beasts.iter())
            {
                let status_icon = if b.bred { "✅" } else { "❌" };
                println!("  {} {}", status_icon, b.name);
            }

            println!(
                "\nBred: {}/{} species ({:.1}%)",
                status.bred_beasts, status.total_beasts, status.progress_percent
            );

            if !status.pool_not_whitelist.is_empty() {
                println!(
                    "\nRegistered in pool but NOT in the 12-species list: {}",
                    status.pool_not_whitelist.join(", ")
                );
            }
            if let Some(sq) = &status.squeeze_indicator {
                println!("\nNOTE: {}", sq);
            }
            println!("\nRoster derived from the save's PFA_26 OneOfEach pool and NamedCreatureDefinition");
            println!("(12 breedable species; phoenix is excluded - not breedable).");
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("Hogwarts Legacy Save Tracker");
    println!("==============================\n");

    println!("Processing save file: {:?}", args.save);
    let (status, beasts) = analyze_all(&args.save)?;

    let format = if args.json {
        OutputFormat::Json
    } else {
        args.format
    };
    let json_format = args.json;

    if args.report == Report::Both {
        if json_format {
            println!(
                "{}",
                serde_json::to_string_pretty(&FullReport {
                    enemies: status,
                    beasts
                })
                .unwrap()
            );
        } else {
            output_status(&status, &format, args.missing_only);
            output_beast_status(&beasts, &format);
        }
    } else if args.report == Report::Enemies {
        output_status(&status, &format, args.missing_only);
    } else {
        output_beast_status(&beasts, &format);
    }

    Ok(())
}
