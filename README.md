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

## Project layout

- `src/main.rs` — the tracker CLI (save parsing, decompression, SQLite, reporting)
- `src/bin/decompile_exe.rs` — a small research helper used while reverse-engineering the save format (not part of the tracker itself)

## Disclaimer

A fan project for personal use, not affiliated with Warner Bros. or Avalanche Software. The tool reads only; it cannot earn achievements for you — use it to know what is left, then go finish them.