# Hogwarts Legacy Achievement Tracker

A small Rust CLI that reads your Hogwarts Legacy save file and reports your progress toward the **Finishing Touches** achievement (some platforms: "Finish Strong" / PFA trophy): land an Ancient Magic finisher on each of the **34** eligible enemy types.

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

## Requirements

- Rust toolchain (stable) — get it from <https://rustup.rs>

That's it. Everything from the game's save format — including the proprietary
**Oodle** compression used for the embedded database — is handled in process with
pure Rust (via the MIT-licensed [`oozextract`] crate), so the tool needs no Oodle DLL
and no files from your game install.

[oozextract]: https://crates.io/crates/oozextract

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

## How it works

1. **GVAS → Oodle → SQLite**: the `.sav` is a UE4 `GVAS` container. The tool locates the `RawDatabaseImage` field, splits it into its ~128 KiB Oodle-compressed package chunks, decompresses each with a pure-Rust implementation of the decompression algorithm (`oozextract`), stitches them back together, and repairs the SQLite header. The chunk format (Kraken full-page blocks) was confirmed byte-identical to the game's own `OodleLZ_Decompress` across ~1000 chunks / 15 saves.
2. **Query `AchievementDynamic` for `PFA_43`** ("Finishing Touches", a *OneOfEach*-type achievement):
   - `Instances` = number of distinct enemy types you've credited (your real progress, e.g. `28`).
   - `OneOfEach` = the registered pool (the 51 seeded special/boss/named entries + every regular class you've hit with an Ancient Magic finisher).
3. **Diff the pool against the 34-class roster** to produce the completed / missing lists. Classes in the pool that aren't on the roster (bosses, named enemies, classmate duels, etc.) are reported separately as "registered in pool but not counted".

## Project layout

- `src/main.rs` — the tracker CLI (save parsing, decompression, SQLite, reporting)
- `src/bin/decompile_exe.rs` — a small research helper used while reverse-engineering the save format (not part of the tracker itself)

## Disclaimer

A fan project for personal use, not affiliated with Warner Bros. or Avalanche Software. The tool reads only; it cannot earn achievements for you — use it to know what is left, then go finish them.