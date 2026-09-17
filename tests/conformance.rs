use std::collections::HashSet;
use std::path::PathBuf;

use hl_save_tracker::analyze_all_with_db;
use hl_save_tracker::analyze_save_with_db;

/// Fixture: a real, sanitized save. Identity strings (character name and
/// character UID) have been scrubbed from the readable header; the rest of
/// the file is byte-for-byte the genuine save captured from the game.
const FIXTURE: &str = "testdata/HL-00-00.sanitized.sav";
const FIXTURE_2: &str = "testdata/HL-00-14.sanitized.sav";
const FIXTURE_3: &str = "testdata/HL-00-07.sanitized.sav";
const FIXTURE_4: &str = "testdata/HL-00-12.sanitized.sav";
const FIXTURE_5: &str = "testdata/HL-00-01.sanitized.sav";

/// Values captured from the real (unsanitized) HL-00-00.sav before scrubbing.
const EXPECTED_INSTANCES: usize = 28;
const EXPECTED_MISSING: [&str; 6] = [
    "DW_Extortionist_Sniper",  // Ashwinder Ranger
    "DW_Extortionist_Captain", // Ashwinder Duellist
    "AnimagusWolf",            // Wolf Animagus (Poacher animagus form)
    "SpiderVenomousSpitter",   // Venomous Shooter
    "Troll_River",             // River Troll
    "DW_Wolf",                 // Mongrel
];
const EXPECTED_NOT_COUNTED: usize = 51; // seeded specials/bosses/named entries

/// "The Nature of the Beast" values captured from HL-00-00.sav:
/// 5 of 12 species bred (Thestral, Puffskein, Mooncalf, Diricawl, Kneazle),
/// none outside the 12-species roster.
const EXPECTED_BRED_BEASTS: usize = 5;
const EXPECTED_MISSING_BEASTS: [&str; 7] = [
    "Fwooper",
    "GiantPurpleToad",
    "Graphorn",
    "Hippogriff",
    "Jobberknoll",
    "Niffler",
    "Unicorn",
];

// Identity data present in the original save; the fixture must not contain them.
const ORIG_NAME: &str = "Cynthia Parter";
const ORIG_UID: &str = "7FC50D4642F86FBD0AF90C886DFDACCD";

/// "Put Down Roots" values captured from HL-00-00.sav: 6 of 8 plants grown
/// (Dittany, Mandrake, ChompingCabbage_Plant, ShrivelFig, Fluxweed, Mallowsweet),
/// none outside the 8-plant roster.
const EXPECTED_GROWN_PLANTS: usize = 6;
const EXPECTED_MISSING_PLANTS: [&str; 2] = ["Knotgrass", "VenomousTentacula"];

/// "Going Through the Potions" values captured from HL-00-00.sav:
/// 5 of 6 potions brewed (Edurus, Maxima, WoundCleaning/Wiggenweld,
/// AMFillPotion/Focus, AutoDamagePotion/Thunderbrew), none outside the
/// 6-potion roster. Invisibility was consumed once but never brewed, so it
/// is missing from the PFA_27 pool.
const EXPECTED_BREWED_POTIONS: usize = 5;
const EXPECTED_MISSING_POTIONS: [&str; 1] = ["InvisibilityPotion"];

/// "Merlin's Beard!" values captured from HL-00-00.sav: 29 of 95 Merlin
/// Trials completed.
const EXPECTED_COMPLETED_TRIALS: usize = 29;

/// "Collector's Edition" values captured from HL-00-00.sav (full DLC-era
/// roster): 571 of 633 collection items obtained across the 10 categories.
const EXPECTED_COLLECTED_ITEMS: usize = 571;
const EXPECTED_TOTAL_ITEMS: usize = 633;

/// "HL-00-14.sav" golden values: the same character further into the game.
/// Finishing Touches went 28→32: Ashwinder Ranger, River Troll, Mongrel
/// (DW_Wolf), and the Poacher Animagus Wolf (credited by the save's counter
/// even though its class wasn't in the whitelist) are the four new finishers.
/// Remaining: Ashwinder Duellist and Venomous Shooter.
const EXPECTED_INSTANCES_2: usize = 32;
const EXPECTED_MISSING_2: [&str; 2] = [
    "DW_Extortionist_Captain", // Ashwinder Duellist
    "SpiderVenomousSpitter",   // Venomous Shooter
];
const EXPECTED_NOT_COUNTED_2: usize = 51; // AnimagusWolf is now whitelisted
const EXPECTED_GROWN_PLANTS_2: usize = 8;
const EXPECTED_BREWED_POTIONS_2: usize = 6;
const EXPECTED_COMPLETED_TRIALS_2: usize = 38;

/// Collector's Edition for HL-00-14.sav (DLC roster now shows Gear 104,
/// 634 total). Conjurations 120/140, Enemies 67/69, Traits 54/75.
const EXPECTED_COLLECTED_ITEMS_2: usize = 583;
const EXPECTED_TOTAL_ITEMS_2: usize = 634;

/// "HL-00-07.sav" golden values: a mid-game point of the same character
/// (Aug 2026), distinct from fixture 1 (start of run) and fixture 2 (end of
/// run). Finishing Touches is 24/34 with a *different* missing set than both
/// other fixtures (Inferius and Venomous Ambusher are done in fixtures 1+2
/// but missing here), pinning down the roster invariant at yet another level.
const EXPECTED_INSTANCES_3: usize = 24;
const EXPECTED_MISSING_3: [&str; 10] = [
    "DW_Extortionist_Sniper",  // Ashwinder Ranger
    "DW_Extortionist_Captain", // Ashwinder Duellist
    "AnimagusWolf",            // Wolf Animagus
    "Inferius",                // Inferius
    "SpiderVenomous",          // Venomous Scurriour
    "SpiderVenomousSpitter",   // Venomous Shooter
    "SpiderVenomousSniper",    // Venomous Ambusher
    "Troll_River",             // River Troll
    "Wolf",                    // Dark Mongrel
    "DW_Wolf",                 // Mongrel
];
const EXPECTED_NOT_COUNTED_3: usize = 51;
const EXPECTED_BRED_BEASTS_3: usize = 1;
const EXPECTED_MISSING_BEASTS_3: [&str; 11] = [
    "Diricawl",
    "Fwooper",
    "GiantPurpleToad",
    "Graphorn",
    "Hippogriff",
    "Jobberknoll",
    "Kneazle",
    "Mooncalf",
    "Niffler",
    "Puffskein",
    "Unicorn",
];
const EXPECTED_GROWN_PLANTS_3: usize = 6;
const EXPECTED_BREWED_POTIONS_3: usize = 5;
const EXPECTED_COMPLETED_TRIALS_3: usize = 21;
const EXPECTED_COLLECTED_ITEMS_3: usize = 460;
const EXPECTED_TOTAL_ITEMS_3: usize = 633;

/// "HL-00-12.sav" golden values: the same character AFTER completing
/// Finishing Touches (34/34). Plants and potions are also complete; beasts
/// are still the same 5/12 as fixture 1, and only Conjurations, Gear, and
/// Traits remain in Collector's Edition.
const EXPECTED_INSTANCES_4: usize = 34;
const EXPECTED_NOT_COUNTED_4: usize = 51;
const EXPECTED_BRED_BEASTS_4: usize = 5;
const EXPECTED_GROWN_PLANTS_4: usize = 8;
const EXPECTED_BREWED_POTIONS_4: usize = 6;
const EXPECTED_COMPLETED_TRIALS_4: usize = 43;
const EXPECTED_COLLECTED_ITEMS_4: usize = 593;
const EXPECTED_TOTAL_ITEMS_4: usize = 634;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

fn fixture_2_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_2)
}

fn fixture_3_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_3)
}

fn fixture_4_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_4)
}

fn assert_sanitized(path: &std::path::Path) {
    let bytes = std::fs::read(path).expect("fixture save missing");
    assert!(bytes.starts_with(b"GVAS"), "not a GVAS save file");
    assert!(bytes.len() > 1_000_000, "suspiciously small save");

    // Identity scrub must have been applied before committing the fixture.
    let labels = bytes
        .windows(ORIG_NAME.len())
        .zip(std::iter::repeat(ORIG_NAME.as_bytes()))
        .filter(|(w, _)| *w == ORIG_NAME.as_bytes())
        .count();
    assert_eq!(
        labels, 0,
        "fixture still contains the original character name"
    );

    let uid_count = bytes
        .windows(ORIG_UID.len())
        .filter(|w| *w == ORIG_UID.as_bytes())
        .count();
    assert_eq!(
        uid_count, 0,
        "fixture still contains the original character UID"
    );
}

#[test]
fn fixtures_are_real_sanitized_saves() {
    assert_sanitized(&fixture_path());
    assert_sanitized(&fixture_2_path());
    assert_sanitized(&fixture_3_path());
    assert_sanitized(&fixture_4_path());
    assert_sanitized(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_5));
}

#[test]
fn conformance_against_real_save_data() {
    let status = analyze_save_with_db(&fixture_path(), std::env::temp_dir().join("hl_ft_test.db"))
        .expect("failed to analyze fixture save");

    assert_eq!(status.achievement_id, "PFA_43");
    assert_eq!(status.total_enemies, 34);
    assert_eq!(status.completed_enemies, EXPECTED_INSTANCES);

    // The roster derivation invariant that holds for every real save:
    // registered whitelist classes must equal the save's Instances count.
    assert_eq!(
        status.completed_enemies_list.len(),
        status.completed_enemies,
        "squeeze detected: registered whitelist classes != Instances"
    );
    assert!(
        status.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );

    let got_missing: HashSet<&str> = status
        .missing_enemies
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    let expected_missing: HashSet<&str> = EXPECTED_MISSING.iter().copied().collect();
    assert_eq!(
        got_missing, expected_missing,
        "missing list diverged from golden values"
    );

    assert_eq!(status.registered_not_counted.len(), EXPECTED_NOT_COUNTED);
}

#[test]
fn conformance_nature_of_the_beast() {
    let full = analyze_all_with_db(
        &fixture_path(),
        std::env::temp_dir().join("hl_beast_test.db"),
    )
    .expect("failed to analyze fixture save");
    let beasts = &full.beasts;

    assert_eq!(beasts.achievement_id, "PFA_26");
    assert_eq!(beasts.total_beasts, 12);
    assert_eq!(beasts.bred_beasts, EXPECTED_BRED_BEASTS);

    // Same roster invariant as Finishing Touches: pool size == Instances.
    assert_eq!(
        beasts.bred_beasts_list.len(),
        beasts.bred_beasts,
        "squeeze detected: registered species != Instances"
    );
    assert!(
        beasts.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );
    assert!(
        beasts.pool_not_whitelist.is_empty(),
        "registered species outside the 12-roster: {:?}",
        beasts.pool_not_whitelist
    );

    let got_missing: HashSet<&str> = beasts
        .missing_beasts
        .iter()
        .map(|b| b.id.as_str())
        .collect();
    let expected_missing: HashSet<&str> = EXPECTED_MISSING_BEASTS.iter().copied().collect();
    assert_eq!(
        got_missing, expected_missing,
        "missing species diverged from golden values"
    );
}

#[test]
fn conformance_put_down_roots() {
    let full = analyze_all_with_db(
        &fixture_path(),
        std::env::temp_dir().join("hl_plant_test.db"),
    )
    .expect("failed to analyze fixture save");
    let plants = &full.plants;

    assert_eq!(plants.achievement_id, "PFA_28");
    assert_eq!(plants.total_plants, 8);
    assert_eq!(plants.grown_plants, EXPECTED_GROWN_PLANTS);

    // Same roster invariant: pool size == Instances.
    assert_eq!(
        plants.grown_plants_list.len(),
        plants.grown_plants,
        "squeeze detected: registered plants != Instances"
    );
    assert!(
        plants.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );
    assert!(
        plants.pool_not_whitelist.is_empty(),
        "registered plants outside the 8-roster: {:?}",
        plants.pool_not_whitelist
    );

    let got_missing: HashSet<&str> = plants
        .missing_plants
        .iter()
        .map(|p| p.id.as_str())
        .collect();
    let expected_missing: HashSet<&str> = EXPECTED_MISSING_PLANTS.iter().copied().collect();
    assert_eq!(
        got_missing, expected_missing,
        "missing plants diverged from golden values"
    );
}

#[test]
fn conformance_going_through_the_potions() {
    let full = analyze_all_with_db(
        &fixture_path(),
        std::env::temp_dir().join("hl_potion_test.db"),
    )
    .expect("failed to analyze fixture save");
    let potions = &full.potions;

    assert_eq!(potions.achievement_id, "PFA_27");
    assert_eq!(potions.total_potions, 6);
    assert_eq!(potions.brewed_potions, EXPECTED_BREWED_POTIONS);

    // Same roster invariant: pool size == Instances.
    assert_eq!(
        potions.brewed_potions_list.len(),
        potions.brewed_potions,
        "squeeze detected: registered potions != Instances"
    );
    assert!(
        potions.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );
    assert!(
        potions.pool_not_whitelist.is_empty(),
        "registered potions outside the 6-roster: {:?}",
        potions.pool_not_whitelist
    );

    let got_missing: HashSet<&str> = potions
        .missing_potions
        .iter()
        .map(|p| p.id.as_str())
        .collect();
    let expected_missing: HashSet<&str> = EXPECTED_MISSING_POTIONS.iter().copied().collect();
    assert_eq!(
        got_missing, expected_missing,
        "missing potions diverged from golden values"
    );
}

#[test]
fn conformance_merlins_beard() {
    let full = analyze_all_with_db(
        &fixture_path(),
        std::env::temp_dir().join("hl_merlin_test.db"),
    )
    .expect("failed to analyze fixture save");
    let merlin = &full.merlin;

    assert_eq!(merlin.achievement_id, "PFA_37");
    assert_eq!(merlin.total_trials, 95);
    assert_eq!(merlin.completed_trials, EXPECTED_COMPLETED_TRIALS);
    assert!(merlin.tracked, "PFA_37 row should exist");

    // Count-based: no roster to validate, just the counter and progress.
    assert_eq!(
        merlin.progress_percent,
        (EXPECTED_COMPLETED_TRIALS as f32 / 95.0) * 100.0
    );
}

#[test]
fn conformance_collectors_edition() {
    let full = analyze_all_with_db(
        &fixture_path(),
        std::env::temp_dir().join("hl_collectors_test.db"),
    )
    .expect("failed to analyze fixture save");
    let collectors = &full.collectors;

    assert_eq!(collectors.achievement_id, "Collection");
    assert_eq!(collectors.achievement_name, "Collector's Edition");
    assert!(collectors.tracked, "CollectionDynamic table should exist");
    assert_eq!(collectors.total_items, EXPECTED_TOTAL_ITEMS);
    assert_eq!(collectors.total_collected, EXPECTED_COLLECTED_ITEMS);
    assert!(
        !collectors.complete,
        "fixture is not a completed collection"
    );
    assert_eq!(
        collectors.progress_percent,
        (EXPECTED_COLLECTED_ITEMS as f32 / EXPECTED_TOTAL_ITEMS as f32) * 100.0
    );

    // The 10 known categories, each with a sane per-category tally.
    assert_eq!(collectors.categories.len(), 10);
    let by_id = |id: &str| {
        collectors
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("missing category {id}"))
    };
    assert_eq!(by_id("Conjurations").obtained, 119);
    assert_eq!(by_id("Conjurations").total, 140);
    assert_eq!(by_id("Enemies").obtained, 64);
    assert_eq!(by_id("Enemies").total, 69);
    assert_eq!(by_id("Traits").obtained, 49);
    assert_eq!(by_id("Traits").total, 75);

    // Fully obtained categories stay complete and sum exactly to the totals.
    for id in [
        "Beasts",
        "Brooms",
        "Exploration",
        "Potions",
        "Seeds",
        "WandStyle",
    ] {
        let c = by_id(id);
        assert!(c.obtained >= c.total, "{id} should be complete: {c:?}");
    }
    assert_eq!(
        collectors
            .categories
            .iter()
            .map(|c| c.obtained)
            .sum::<usize>(),
        EXPECTED_COLLECTED_ITEMS
    );
    assert_eq!(
        collectors.categories.iter().map(|c| c.total).sum::<usize>(),
        EXPECTED_TOTAL_ITEMS
    );
}

/// Golden values for the second fixture, HL-00-14.sav. Fixture 1 is an early
/// save; this one is the same character further along, so every achievement
/// either shows more progress or is complete.
#[test]
fn conformance_hl_00_14_progress() {
    let full = analyze_all_with_db(
        &fixture_2_path(),
        std::env::temp_dir().join("hl_14_test.db"),
    )
    .expect("failed to analyze HL-00-14 fixture");

    // Finishing Touches
    assert_eq!(full.enemies.total_enemies, 34);
    assert_eq!(full.enemies.completed_enemies, EXPECTED_INSTANCES_2);
    assert_eq!(
        full.enemies.completed_enemies_list.len(),
        full.enemies.completed_enemies,
        "squeeze detected: registered whitelist classes != Instances"
    );
    assert!(
        full.enemies.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );
    let got_missing: HashSet<&str> = full
        .enemies
        .missing_enemies
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    let expected_missing: HashSet<&str> = EXPECTED_MISSING_2.iter().copied().collect();
    assert_eq!(
        got_missing, expected_missing,
        "missing list diverged from golden values"
    );
    assert_eq!(
        full.enemies.registered_not_counted.len(),
        EXPECTED_NOT_COUNTED_2
    );

    // Beasts are unchanged since fixture 1.
    assert_eq!(full.beasts.bred_beasts, EXPECTED_BRED_BEASTS);
    assert_eq!(
        full.beasts.missing_beasts.len(),
        EXPECTED_MISSING_BEASTS.len()
    );
    assert!(full.beasts.pool_not_whitelist.is_empty());

    // Put Down Roots and Going Through the Potions are now complete.
    assert_eq!(full.plants.grown_plants, EXPECTED_GROWN_PLANTS_2);
    assert!(full.plants.missing_plants.is_empty());
    assert!(full.plants.pool_not_whitelist.is_empty());
    assert_eq!(full.potions.brewed_potions, EXPECTED_BREWED_POTIONS_2);
    assert!(full.potions.missing_potions.is_empty());
    assert!(full.potions.pool_not_whitelist.is_empty());

    // Merlin's Beard
    assert_eq!(full.merlin.completed_trials, EXPECTED_COMPLETED_TRIALS_2);
    assert_eq!(
        full.merlin.progress_percent,
        (EXPECTED_COMPLETED_TRIALS_2 as f32 / 95.0) * 100.0
    );

    // Collector's Edition
    assert_eq!(full.collectors.total_items, EXPECTED_TOTAL_ITEMS_2);
    assert_eq!(full.collectors.total_collected, EXPECTED_COLLECTED_ITEMS_2);
    assert!(!full.collectors.complete, "HL-00-14 is not complete yet");
    assert_eq!(full.collectors.categories.len(), 10);
    let by_id = |id: &str| {
        full.collectors
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("missing category {id}"))
    };
    assert_eq!(by_id("Conjurations").obtained, 120);
    assert_eq!(by_id("Conjurations").total, 140);
    assert_eq!(by_id("Enemies").obtained, 67);
    assert_eq!(by_id("Enemies").total, 69);
    assert_eq!(by_id("Traits").obtained, 54);
    assert_eq!(by_id("Traits").total, 75);
    for id in [
        "Beasts",
        "Brooms",
        "Exploration",
        "Potions",
        "Seeds",
        "WandStyle",
    ] {
        let c = by_id(id);
        assert!(c.obtained >= c.total, "{id} should be complete: {c:?}");
    }
    assert_eq!(
        full.collectors
            .categories
            .iter()
            .map(|c| c.obtained)
            .sum::<usize>(),
        EXPECTED_COLLECTED_ITEMS_2
    );
    assert_eq!(
        full.collectors
            .categories
            .iter()
            .map(|c| c.total)
            .sum::<usize>(),
        EXPECTED_TOTAL_ITEMS_2
    );
}

/// Golden values for the third fixture, HL-00-07.sav. A mid-game point with a
/// missing set distinct from fixtures 1 and 2, exercising the roster
/// invariants ("registered == Instances", empty pool_not_whitelist) at yet
/// another progress level.
#[test]
fn conformance_hl_00_07_midgame() {
    let full = analyze_all_with_db(
        &fixture_3_path(),
        std::env::temp_dir().join("hl_07_test.db"),
    )
    .expect("failed to analyze HL-00-07 fixture");

    // Finishing Touches
    assert_eq!(full.enemies.total_enemies, 34);
    assert_eq!(full.enemies.completed_enemies, EXPECTED_INSTANCES_3);
    assert_eq!(
        full.enemies.completed_enemies_list.len(),
        full.enemies.completed_enemies,
        "squeeze detected: registered whitelist classes != Instances"
    );
    assert!(
        full.enemies.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );
    let got_missing: HashSet<&str> = full
        .enemies
        .missing_enemies
        .iter()
        .map(|e| e.id.as_str())
        .collect();
    let expected_missing: HashSet<&str> = EXPECTED_MISSING_3.iter().copied().collect();
    assert_eq!(
        got_missing, expected_missing,
        "missing list diverged from golden values"
    );
    assert_eq!(
        full.enemies.registered_not_counted.len(),
        EXPECTED_NOT_COUNTED_3
    );

    // The Nature of the Beast: only the Thestral bred so far.
    assert_eq!(full.beasts.bred_beasts, EXPECTED_BRED_BEASTS_3);
    assert_eq!(
        full.beasts.missing_beasts.len(),
        EXPECTED_MISSING_BEASTS_3.len()
    );
    assert!(full.beasts.pool_not_whitelist.is_empty());
    let got_missing_beasts: HashSet<&str> = full
        .beasts
        .missing_beasts
        .iter()
        .map(|b| b.id.as_str())
        .collect();
    let expected_missing_beasts: HashSet<&str> =
        EXPECTED_MISSING_BEASTS_3.iter().copied().collect();
    assert_eq!(
        got_missing_beasts, expected_missing_beasts,
        "missing species diverged from golden values"
    );

    // Put Down Roots and Going Through the Potions: one step from complete.
    assert_eq!(full.plants.grown_plants, EXPECTED_GROWN_PLANTS_3);
    assert_eq!(
        full.plants.missing_plants.len(),
        EXPECTED_MISSING_PLANTS.len(),
        "missing plants diverged from golden values"
    );
    assert!(full.plants.pool_not_whitelist.is_empty());
    assert_eq!(full.potions.brewed_potions, EXPECTED_BREWED_POTIONS_3);
    assert_eq!(
        full.potions.missing_potions.len(),
        EXPECTED_MISSING_POTIONS.len(),
        "missing potions diverged from golden values"
    );
    assert!(full.potions.pool_not_whitelist.is_empty());

    // Merlin's Beard
    assert_eq!(full.merlin.completed_trials, EXPECTED_COMPLETED_TRIALS_3);
    assert_eq!(
        full.merlin.progress_percent,
        (EXPECTED_COMPLETED_TRIALS_3 as f32 / 95.0) * 100.0
    );

    // Collector's Edition
    assert_eq!(full.collectors.total_items, EXPECTED_TOTAL_ITEMS_3);
    assert_eq!(full.collectors.total_collected, EXPECTED_COLLECTED_ITEMS_3);
    assert!(!full.collectors.complete, "HL-00-07 is not complete yet");
    assert_eq!(full.collectors.categories.len(), 10);
    let by_id = |id: &str| {
        full.collectors
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("missing category {id}"))
    };
    assert_eq!(by_id("Conjurations").obtained, 85);
    assert_eq!(by_id("Conjurations").total, 140);
    assert_eq!(by_id("Enemies").obtained, 53);
    assert_eq!(by_id("Enemies").total, 69);
    assert_eq!(by_id("Traits").obtained, 37);
    assert_eq!(by_id("Traits").total, 75);
    for id in ["Potions", "Seeds"] {
        let c = by_id(id);
        assert!(c.obtained >= c.total, "{id} should be complete: {c:?}");
    }
    assert_eq!(
        full.collectors
            .categories
            .iter()
            .map(|c| c.obtained)
            .sum::<usize>(),
        EXPECTED_COLLECTED_ITEMS_3
    );
    assert_eq!(
        full.collectors
            .categories
            .iter()
            .map(|c| c.total)
            .sum::<usize>(),
        EXPECTED_TOTAL_ITEMS_3
    );
}

/// Golden values for the fourth fixture, HL-00-12.sav: Finishing Touches is
/// fully completed (34/34). Pins down the "registered == Instances" invariant
/// at the completion boundary, plus post-completion values for every other
/// tracked achievement.
#[test]
fn conformance_hl_00_12_complete() {
    let full = analyze_all_with_db(
        &fixture_4_path(),
        std::env::temp_dir().join("hl_12_test.db"),
    )
    .expect("failed to analyze HL-00-12 fixture");

    // Finishing Touches is complete.
    assert_eq!(full.enemies.total_enemies, 34);
    assert_eq!(full.enemies.completed_enemies, EXPECTED_INSTANCES_4);
    assert_eq!(
        full.enemies.completed_enemies_list.len(),
        full.enemies.completed_enemies,
        "squeeze detected: registered whitelist classes != Instances"
    );
    assert!(
        full.enemies.missing_enemies.is_empty(),
        "finishing touches done"
    );
    assert!(
        full.enemies.squeeze_indicator.is_none(),
        "unexpected squeeze indicator"
    );
    assert_eq!(
        full.enemies.registered_not_counted.len(),
        EXPECTED_NOT_COUNTED_4
    );

    // Beasts: same 5/12 as fixture 1.
    assert_eq!(full.beasts.bred_beasts, EXPECTED_BRED_BEASTS_4);
    assert_eq!(
        full.beasts.missing_beasts.len(),
        EXPECTED_MISSING_BEASTS.len()
    );
    assert!(full.beasts.pool_not_whitelist.is_empty());

    // Plants and potions are complete.
    assert_eq!(full.plants.grown_plants, EXPECTED_GROWN_PLANTS_4);
    assert!(full.plants.missing_plants.is_empty());
    assert!(full.plants.pool_not_whitelist.is_empty());
    assert_eq!(full.potions.brewed_potions, EXPECTED_BREWED_POTIONS_4);
    assert!(full.potions.missing_potions.is_empty());
    assert!(full.potions.pool_not_whitelist.is_empty());

    // Merlin's Beard
    assert_eq!(full.merlin.completed_trials, EXPECTED_COMPLETED_TRIALS_4);
    assert_eq!(
        full.merlin.progress_percent,
        (EXPECTED_COMPLETED_TRIALS_4 as f32 / 95.0) * 100.0
    );

    // Collector's Edition
    assert_eq!(full.collectors.total_items, EXPECTED_TOTAL_ITEMS_4);
    assert_eq!(full.collectors.total_collected, EXPECTED_COLLECTED_ITEMS_4);
    assert!(
        !full.collectors.complete,
        "HL-00-12 is not full collection yet"
    );
    assert_eq!(full.collectors.categories.len(), 10);
    let by_id = |id: &str| {
        full.collectors
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("missing category {id}"))
    };
    assert_eq!(by_id("Conjurations").obtained, 121);
    assert_eq!(by_id("Conjurations").total, 140);
    assert_eq!(by_id("Gear").obtained, 98);
    assert_eq!(by_id("Gear").total, 104);
    assert_eq!(by_id("Traits").obtained, 59);
    assert_eq!(by_id("Traits").total, 75);
    for id in [
        "Beasts",
        "Brooms",
        "Enemies",
        "Exploration",
        "Potions",
        "Seeds",
        "WandStyle",
    ] {
        let c = by_id(id);
        assert_eq!(c.obtained, c.total, "{id} should be complete: {c:?}");
    }
    assert_eq!(
        full.collectors
            .categories
            .iter()
            .map(|c| c.obtained)
            .sum::<usize>(),
        EXPECTED_COLLECTED_ITEMS_4
    );
    assert_eq!(
        full.collectors
            .categories
            .iter()
            .map(|c| c.total)
            .sum::<usize>(),
        EXPECTED_TOTAL_ITEMS_4
    );
}

#[test]
fn conformance_hl_00_01_breeding_pairs() {
    let full = analyze_all_with_db(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_5),
        std::env::temp_dir().join("hl_01_breeding_test.db"),
    )
    .expect("failed to analyze HL-00-01 fixture");

    assert_eq!(full.enemies.completed_enemies, 34);
    assert_eq!(full.enemies.completed_enemies_list.len(), 34);
    assert!(full.enemies.missing_enemies.is_empty());
    assert!(full.enemies.squeeze_indicator.is_none());
    assert_eq!(full.enemies.registered_not_counted.len(), 51);

    let beasts = &full.beasts;
    assert!(beasts.tracked);
    assert_eq!(beasts.total_beasts, 12);
    assert_eq!(beasts.bred_beasts, 10);
    assert_eq!(beasts.bred_beasts_list.len(), 10);
    assert!(beasts.pool_not_whitelist.is_empty());
    assert!(beasts.squeeze_indicator.is_none());
    let missing: HashSet<_> = beasts
        .missing_beasts
        .iter()
        .map(|b| b.id.as_str())
        .collect();
    assert_eq!(missing, HashSet::from(["Graphorn", "Unicorn"]));
    for (id, males, females, pair_status) in [
        ("Diricawl", 1, 1, "Pair owned"),
        ("Fwooper", 2, 2, "Pair owned"),
        ("GiantPurpleToad", 1, 1, "Pair owned"),
        ("Graphorn", 1, 0, "Missing female"),
        ("Hippogriff", 2, 2, "Pair owned"),
        ("Jobberknoll", 4, 2, "Pair owned"),
        ("Kneazle", 2, 2, "Pair owned"),
        ("Mooncalf", 6, 4, "Pair owned"),
        ("Niffler", 2, 1, "Pair owned"),
        ("Puffskein", 3, 2, "Pair owned"),
        ("Thestral", 1, 3, "Pair owned"),
        ("Unicorn", 0, 3, "Missing male"),
    ] {
        let beast = beasts
            .bred_beasts_list
            .iter()
            .chain(beasts.missing_beasts.iter())
            .find(|b| b.id == id)
            .unwrap();
        let owned = beast.owned.as_ref().expect("ownership data missing");
        assert_eq!(owned.adult_males, males, "{id} male count");
        assert_eq!(owned.adult_females, females, "{id} female count");
        assert_eq!(owned.unknown_gender, 0, "{id} unknown sex count");
        assert_eq!(owned.pair_status(), pair_status, "{id} pair status");
        assert_eq!(beast.bred, !missing.contains(id), "{id} breeding progress");
    }

    assert_eq!(full.plants.grown_plants, 8);
    assert!(full.plants.missing_plants.is_empty());
    assert_eq!(full.potions.brewed_potions, 6);
    assert!(full.potions.missing_potions.is_empty());
    assert_eq!(full.merlin.completed_trials, 54);
    assert_eq!(full.merlin.total_trials, 95);
    assert_eq!(full.collectors.total_collected, 600);
    assert_eq!(full.collectors.total_items, 634);
    assert!(!full.collectors.complete);
    assert_eq!(full.collectors.categories.len(), 10);
    for (id, obtained, total) in [
        ("Beasts", 13, 13),
        ("Brooms", 15, 15),
        ("Conjurations", 122, 140),
        ("Enemies", 69, 69),
        ("Exploration", 150, 150),
        ("Gear", 99, 104),
        ("Potions", 10, 10),
        ("Seeds", 16, 16),
        ("Traits", 64, 75),
        ("WandStyle", 42, 42),
    ] {
        let category = full
            .collectors
            .categories
            .iter()
            .find(|c| c.id == id)
            .unwrap();
        assert_eq!(category.obtained, obtained, "{id} obtained");
        assert_eq!(category.total, total, "{id} total");
    }
}
