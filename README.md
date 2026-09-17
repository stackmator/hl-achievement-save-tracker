# Hogwarts Legacy Save Tracker

A small Rust CLI that reads your Hogwarts Legacy save file and reports progress toward the achievements it currently supports. Everything is derived from the achievement-tracking data the game records in the save itself — no guides or guesswork.

```
Hogwarts Legacy Save Tracker
==============================

Processing save file: "…\HL-00-00.sav"

=== Finishing Touches ===
Progress: 28/34 (82.4%)

--- Trolls ---
  ✅ Forest Troll
  ✅ Mountain Troll

--- Ashwinders ---
  ❌ Ashwinder Ranger
  ❌ Ashwinder Duellist
  ...

Total: 28/34 enemies completed (82.4%)

=== The Nature of the Beast ===
Progress: 5/12 (41.7%)

  ✅ Diricawl
  ✅ Puffskein
  ✅ Thestral
  ❌ Fwooper
  ❌ Hippogriff
  ...

Bred: 5/12 species (41.7%)

=== Put Down Roots ===
Progress: 6/8 (75.0%)

  ✅ Mandrake
  ✅ Dittany
  ❌ Knotgrass
  ❌ Venomous Tentacula

Grown: 6/8 plants (75.0%)

=== Going Through the Potions ===
Progress: 5/6 (83.3%)

  ✅ Edurus Potion
  ✅ Maxima Potion
  ✅ Wiggenweld Potion
  ❌ Invisibility Potion

Brewed: 5/6 potions (83.3%)

=== Merlin's Beard! ===
Progress: 29/95 (30.5%)

Completed: 29/95 Merlin Trials (30.5%)

=== Collector's Edition ===
Progress: 571/633 (90.2%)

  ✅ Beasts         13/13 (100.0%)
  ✅ Brooms         15/15 (100.0%)
  ❌ Conjurations   119/140 (85.0%)
  ❌ Enemies        64/69 (92.8%)
  ✅ Exploration    150/150 (100.0%)
  ❌ Gear           93/103 (90.3%)
  ✅ Potions        10/10 (100.0%)
  ✅ Seeds          16/16 (100.0%)
  ❌ Traits         49/75 (65.3%)
  ✅ Wand Handles   42/42 (100.0%)

Collected: 571/633 items (90.2%)
```

Unlike guide-based estimators, this tool reads the **actual tracking data from your save**, so the count and per-enemy list reflect exactly what the game recorded.

## Supported achievements

### Finishing Touches

Land an Ancient Magic finisher on each of the **34** eligible enemy types ("Finish Strong" on some platforms). The tracker reports your completed count (e.g. `28/34`) and lists which enemy classes are done and which are still needed.

### The Nature of the Beast

Breed each of the **12** breedable beast species (phoenixes don't count). The tracker reads the same `OneOfEach` registration data the game records per species, reporting your bred count (e.g. `5/12`) and flagging any surprise pool entries.

The beast report also reads `NurturingCreatureDynamic` to count owned **adult males and females** across your inventory and all four Room of Requirement vivariums. Offspring and classroom beasts are excluded. Each species shows `Pair owned`, `Missing male`, `Missing female`, or `Missing male and female`; unknown sex or unavailable ownership data is reported explicitly rather than assumed missing.

**Pair owned does not mean ready to breed:** both adults must be together in a vivarium with a breeding pen. Current ownership is separate from the historical achievement progress, so a previously bred species can still be missing a partner now.

For example, the `HL-00-01` conformance fixture has **10/12 species bred**, with a **female Graphorn** and a **male Unicorn** still needed.

```text
Graphorn - Adult males: 1, adult females: 0, unknown sex: 0 - Missing female
Unicorn - Adult males: 0, adult females: 3, unknown sex: 0 - Missing male
```

JSON includes `owned.adult_males`, `owned.adult_females`, and `owned.unknown_gender` for each species (`owned: null` when ownership data is unavailable). CSV includes these counts and a `pair_status` column.

### Put Down Roots

Grow each of the **8** types of plant in the Room of Requirement (Dittany, Fluxweed, Knotgrass, Mallowsweet, Mandrake, Shrivelfig, Chinese Chomping Cabbage, Venomous Tentacula). Reports your grown count (e.g. `6/8`) and flags any surprise pool entries.

### Going Through the Potions

Brew each of the **6** types of potion (Wiggenweld, Edurus, Maxima, Focus, Invisibility, Thunderbrew). Reports your brewed count (e.g. `5/6`) and flags any surprise pool entries.

### Merlin's Beard!

Complete all **95** Merlin Trials. The save records only the total completed count, so the tracker reports your progress (`29/95`); individual trials aren't tracked in the save data.

### Collector's Edition

Obtain every item in all **10** collection categories (Conjurations, Enemies, Exploration, Gear, Traits, Wand Handles, plus Beasts, Brooms, Potions, Seeds). Unlike the other achievements this is tracked through the save's `CollectionDynamic` event ledger rather than a single counter, so the tracker counts **distinct items** that have an "Obtained" entry per category (potions can rack up dozens of duplicate pickup rows) and reports a table of every category. Totals come from the save's own collection roster, so DLC-era saves show slightly larger totals than earlier builds (e.g. Gear 103 vs 97).

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
| `--report <which>` | Which achievements: `both` (default), `enemies`, `beasts`, `plants`, `potions`, `merlin`, `collectors` |
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

# Just the collections progress (per-category table)
hl-save-tracker.exe -s "…\HL-00-00.sav" --report collectors

# Write the report to a file instead of stdout
hl-save-tracker.exe -s "…\HL-00-00.sav" -o report.txt
```

To show breeding progress and missing partners:

```powershell
hl-save-tracker.exe -s "…\HL-00-01.sav" --report beasts
```

Saves live in `%LOCALAPPDATA%\Hogwarts Legacy\Saved\SaveGames\<SteamID>\`. Choose the most recently modified `HL-*.sav`; `HL-00-00.sav` is not necessarily the latest save. The file is read-only; the tool never modifies your save.

## Tests

```powershell
cargo test
cargo check --all-targets
cargo clippy --all-targets
```

`tests/conformance.rs` checks five sanitized real-save fixtures in `testdata/`. The `HL-00-01.sanitized.sav` fixture covers breeding progress, adult male/female counts and missing-partner status for all 12 breedable species, plus progress for the other supported achievements. Unit tests cover ownership filtering, unknown sex, unavailable data, and table/JSON/CSV output.

## Project layout

- `src/main.rs` — the tracker CLI (save parsing, decompression, SQLite, reporting)
- `src/lib.rs` — save → SQLite extraction pipeline and the public analysis API
- `src/achievements/` — one module per supported achievement (`finishing_touches.rs`, `nature_of_the_beast.rs`, `put_down_roots.rs`, `going_through_the_potions.rs`, `merlins_beard.rs`, `collectors_edition.rs`)
- `src/bin/decompile_exe.rs` — a small research helper used while reverse-engineering the save format (not part of the tracker itself)
- `src/bin/sanitize_save.rs` — scrubs identity data (character name / UID) from a save to produce a commit-safe test fixture

## Disclaimer

A fan project for personal use, not affiliated with Warner Bros. or Avalanche Software. The tool reads only; it cannot earn achievements for you — use it to know what is left, then go finish them.