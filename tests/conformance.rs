use std::collections::HashSet;
use std::path::PathBuf;

use hl_save_tracker::analyze_all_with_db;
use hl_save_tracker::analyze_save_with_db;

/// Fixture: a real, sanitized save. Identity strings (character name and
/// character UID) have been scrubbed from the readable header; the rest of
/// the file is byte-for-byte the genuine save captured from the game.
const FIXTURE: &str = "testdata/HL-00-00.sanitized.sav";

/// Values captured from the real (unsanitized) HL-00-00.sav before scrubbing.
const EXPECTED_INSTANCES: usize = 28;
const EXPECTED_MISSING: [&str; 6] = [
    "DW_Extortionist_Sniper",  // Ashwinder Ranger
    "DW_Extortionist_Captain", // Ashwinder Captain
    "DW_Poacher_Captain",      // Poacher Captain
    "SpiderVenomousSpitter",   // Venomous Ambusher
    "Troll_River",             // River Troll
    "DW_Wolf",                 // Dark Mongrel
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

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE)
}

#[test]
fn fixture_is_a_real_sanitized_save() {
    let bytes = std::fs::read(fixture_path()).expect("fixture save missing");
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
    let (_, beasts) = analyze_all_with_db(
        &fixture_path(),
        std::env::temp_dir().join("hl_beast_test.db"),
    )
    .expect("failed to analyze fixture save");

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
