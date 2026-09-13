use std::path::PathBuf;
use clap::Parser;
use hl_save_tracker::{AchievementStatus, analyze_save};

#[derive(Parser)]
#[command(name = "hl-save-tracker", version, about = "Track Hogwarts Legacy achievements from save files")]
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
}

#[derive(Debug, clap::ValueEnum, Clone, Default)]
enum OutputFormat {
    #[default]
    Table,
    Json,
    Csv,
}

fn output_status(status: &AchievementStatus, format: OutputFormat, missing_only: bool) {
    match format {
        OutputFormat::Json => {
            if missing_only {
                println!("{}", serde_json::to_string_pretty(&status.missing_enemies).unwrap());
            } else {
                println!("{}", serde_json::to_string_pretty(status).unwrap());
            }
        }
        OutputFormat::Csv => {
            println!("id,name,category,candidate,registered,completed");
            let enemies: Vec<&hl_save_tracker::EnemyStatus> = if missing_only {
                status.missing_enemies.iter().collect()
            } else {
                status.completed_enemies_list.iter().chain(status.missing_enemies.iter()).collect()
            };
            for e in enemies {
                println!("{},{},{},{},{},{}", e.id, e.name, e.category, e.candidate, e.registered, e.completed);
            }
        }
        OutputFormat::Table => {
            println!("\n=== {} ({}) ===", status.achievement_name, status.achievement_id);
            println!("Progress: {}/{} ({:.1}%)", status.completed_enemies, status.total_enemies, status.progress_percent);
            println!();

            let enemies: Vec<&hl_save_tracker::EnemyStatus> = if missing_only {
                status.missing_enemies.iter().collect()
            } else {
                status.completed_enemies_list.iter().chain(status.missing_enemies.iter()).collect()
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

            println!("\nTotal: {}/{} enemies completed ({:.1}%)",
                status.completed_enemies, status.total_enemies, status.progress_percent);

            if !status.registered_not_counted.is_empty() {
                println!("\nRegistered in pool but NOT in the 34-class list (named/boss/one-offs): {}",
                    status.registered_not_counted.len());
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

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("Hogwarts Legacy Save Tracker - Finishing Touches Achievement");
    println!("=============================================================\n");

    println!("Processing save file: {:?}", args.save);
    let status = analyze_save(&args.save)?;

    let format = if args.json { OutputFormat::Json } else { args.format };
    output_status(&status, format, args.missing_only);

    Ok(())
}