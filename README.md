# Hogwarts Legacy Save Tracker

A small Rust CLI that reads your Hogwarts Legacy save file and reports progress toward the achievements it currently supports. Everything is derived from the achievement-tracking data the game records in the save itself — no guides or guesswork.

```
=== Finishing Touches ===
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

### Finishing Touches

Land an Ancient Magic finisher on each of the **34** eligible enemy types ("Finish Strong" on some platforms). The tracker reports your completed count (e.g. `28/34`) and lists which enemy classes are done and which are still needed.

### The Nature of the Beast

Breed each of the **12** breedable beast species (phoenixes don't count). The tracker reads the same `OneOfEach` registration data the game records per species, reporting your bred count (e.g. `5/12`) and flagging any surprise pool entries.

### Put Down Roots

Grow each of the **8** types of plant in the Room of Requirement (Dittany, Fluxweed, Knotgrass, Mallowsweet, Mandrake, Shrivelfig, Chinese Chomping Cabbage, Venomous Tentacula). Reports your grown count (e.g. `6/8`) and flags any surprise pool entries.

### Going Through the Potions

Brew each of the **6** types of potion (Wiggenweld, Edurus, Maxima, Focus, Invisibility, Thunderbrew). Reports your brewed count (e.g. `5/6`) and flags any surprise pool entries.

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
| `--report <which>` | Which achievements: `both` (default), `enemies`, `beasts`, `plants`, `potions` |
| `--json` | Shorthand for `--format json` |
| `-o, --out <PATH>` | Write the report to a file instead of stdout |

### Examples

```powershell
# Full table report
hl-save-tracker.exe -s "…\HL-00-00.sav"

# Just what's left, as JSON (good for scripts / dashboards)
hl-save-tracker.exe -s "…\HL-00-00.sav" --missing-only --json

# CSV of everything
hl-save-tracker.exe -s "…\HL-00-00.sav" -f csv

# Write the report to a file instead of stdout
hl-save-tracker.exe -s "…\HL-00-00.sav" -o report.txt
```

Saves live in `%LOCALAPPDATA%\Hogwarts Legacy\Saved\SaveGames\<SteamID>\`. The file is read-only; the tool never modifies your save.

## Project layout

- `src/main.rs` — the tracker CLI (save parsing, decompression, SQLite, reporting)
- `src/lib.rs` — save → SQLite extraction pipeline and the public analysis API
- `src/achievements/` — one module per supported achievement (`finishing_touches.rs`, `nature_of_the_beast.rs`, `put_down_roots.rs`)
- `src/bin/decompile_exe.rs` — a small research helper used while reverse-engineering the save format (not part of the tracker itself)
- `src/bin/sanitize_save.rs` — scrubs identity data (character name / UID) from a save to produce a commit-safe test fixture

## Disclaimer

A fan project for personal use, not affiliated with Warner Bros. or Avalanche Software. The tool reads only; it cannot earn achievements for you — use it to know what is left, then go finish them.