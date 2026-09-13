# Hogwarts Legacy Save Tracker

A small Rust CLI that reads your Hogwarts Legacy save file and reports progress toward the achievements it currently supports. Everything is derived from the achievement-tracking data the game records in the save itself — no guides or guesswork.

```
=== Finishing Touches (PFA_43) ===
Progress: 28/34 (82.4%)

--- Trolls ---
  ✅ Forest Troll
  ✅ Mountain Troll

--- Ashwinders ---
  ❌ Ashwinder Ranger
  ❌ Ashwinder Captain
  ...
```

Unlike guide-based estimators, this tool reads the **actual tracking data from your save**, so the count and per-enemy list reflect exactly what the game recorded.

## Supported achievements

### Finishing Touches (PFA_43)

Land an Ancient Magic finisher on each of the **34** eligible enemy types ("Finish Strong" on some platforms, the platform trophy for PFA_43). The tracker reports your completed count (e.g. `28/34`) and lists which enemy classes are done and which are still needed.

### The 34 enemy types

Derived from the game's own data tables (`PhoenixGameData.sqlite`), not from forum lists:

## Build

```powershell
cargo build --release
# binary at target\release\hl-save-tracker.exe
```

## Usage

```powershell
hl-save-tracker.exe --save "C:\Users\<you>\AppData\Local\Hogwarts Legacy\Saved\SaveGames\<SteamID>\HL-00-00.sav"
```

### Options

| Flag | Description |
| --- | --- |
| `-s, --save <PATH>` | Path to a `.sav` file (**required**) |
| `-f, --format <fmt>` | Output: `table` (default), `json`, `csv` |
| `--missing-only` | Show only the enemy types still needed |
| `--json` | Shorthand for `--format json` |

### Examples

```powershell
# Full table report
hl-save-tracker.exe -s "…\HL-00-00.sav"

# Just what's left, as JSON (good for scripts / dashboards)
hl-save-tracker.exe -s "…\HL-00-00.sav" --missing-only --json

# CSV of everything
hl-save-tracker.exe -s "…\HL-00-00.sav" -f csv
```

Saves live in `%LOCALAPPDATA%\Hogwarts Legacy\Saved\SaveGames\<SteamID>\`. The file is read-only; the tool never modifies your save.

## The 34 enemy types

Derived from the game's own data tables (`PhoenixGameData.sqlite`), not from forum lists:

| Category | Count | Classes |
| --- | --- | --- |
| Ashwinders | 6 | Ashwinder, Executioner, Duellist, Ranger, Tank, Captain |
| Poachers | 6 | Poacher, Executioner, Duellist, Ranger, Tank, Captain |
| Goblin Loyalists | 4 | Assassin, Warrior, Sentinel, Ranger |
| Inferi | 1 | Inferius |
| Dugbogs | 3 | Coastal, Lake, Marsh |
| Spiders | 9 | Thornback Scurriour/Ambusher/Matriarch/Shooter, Acromantula, Venomous Scurriour/Ambusher/Shooter/Matriarch |
| Trolls | 3 | Forest, Mountain, River |
| Mongrels | 2 | Mongrel, Dark Mongrel |

Notable exclusions (they appear in the game's internal achievement pool so they can look "complete", but the game never counts them): **Armored Troll** and **Loyalist Commander/Chieftain**. Both are seeded into the tracking pool on a new character and never contribute to the 34.

## Project layout

- `src/main.rs` — the tracker CLI (save parsing, decompression, SQLite, reporting)
- `src/bin/decompile_exe.rs` — a small research helper used while reverse-engineering the save format (not part of the tracker itself)

## Disclaimer

A fan project for personal use, not affiliated with Warner Bros. or Avalanche Software. The tool reads only; it cannot earn achievements for you — use it to know what is left, then go finish them.