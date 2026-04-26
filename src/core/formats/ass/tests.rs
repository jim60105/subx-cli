//! Co-located unit tests for the ASS/SSA format module.

use super::AssFormat;
use super::parser::{self, MAX_CUE_BYTES};
use super::time::parse_ass_time;
use crate::core::formats::SubtitleFormat;
use std::time::Duration;

const SAMPLE_ASS: &str = "[Script Info]\nScriptType: v4.00+\n\n[V4+ Styles]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n\n[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,Hello\\NASS\n";

#[test]
fn test_detect_ass() {
    let fmt = AssFormat;
    assert!(fmt.detect(SAMPLE_ASS));
    assert!(!fmt.detect("Not ASS content"));
}

#[test]
fn test_parse_ass_and_serialize() {
    let fmt = AssFormat;
    let subtitle = fmt.parse(SAMPLE_ASS).expect("ASS parse failed");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Hello\nASS");
    let out = fmt.serialize(&subtitle).expect("ASS serialize failed");
    assert!(out.contains("Dialogue: 0,0:00:01.00,0:00:02.50"));
    assert!(out.contains("Hello\\NASS"));
}

#[test]
fn test_parse_ass_missing_start_field() {
    let fmt = AssFormat;
    // Format line omits the Start field.
    let content = "[Events]\nFormat: Layer,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:02.50,Default,,0000,0000,0000,,Hi\n";
    let err = fmt.parse(content).expect_err("missing Start must error");
    let msg = err.to_string();
    assert!(msg.contains("Start"), "unexpected error: {}", msg);
}

#[test]
fn test_parse_ass_missing_end_field() {
    let fmt = AssFormat;
    let content = "[Events]\nFormat: Layer,Start,Style,Name,MarginL,MarginR,MarginV,Effect,Text\nDialogue: 0,0:00:01.00,Default,,0000,0000,0000,,Hi\n";
    let err = fmt.parse(content).expect_err("missing End must error");
    assert!(err.to_string().contains("End"));
}

#[test]
fn test_parse_ass_missing_text_field() {
    let fmt = AssFormat;
    let content = "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect\nDialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,\n";
    let err = fmt.parse(content).expect_err("missing Text must error");
    assert!(err.to_string().contains("Text"));
}

#[test]
fn test_parse_ass_time_overflow() {
    // Hours value near u64::MAX to force checked_mul overflow.
    let overflow_time = format!("{}:00:00.00", u64::MAX);
    let err = parse_ass_time(&overflow_time).expect_err("overflow must be detected");
    assert!(err.to_string().to_lowercase().contains("overflow"));
}

#[test]
fn test_parse_ass_time_valid() {
    let d = parse_ass_time("1:02:03.45").expect("valid time");
    assert_eq!(
        d,
        Duration::from_millis(3_600_000 + 2 * 60_000 + 3 * 1000 + 450)
    );
}

// ----- Hardening (subtitle-parser-hardening matrix) -----

#[test]
fn test_parse_ass_empty_input_rejected() {
    let fmt = AssFormat;
    let err = fmt.parse("").expect_err("empty input must error");
    assert!(matches!(
        err,
        crate::error::SubXError::SubtitleFormat { .. }
    ));
    let err2 = fmt
        .parse("   \n\t\n")
        .expect_err("whitespace-only input must error");
    assert!(matches!(
        err2,
        crate::error::SubXError::SubtitleFormat { .. }
    ));
}

#[test]
fn test_parse_ass_missing_events_section_rejected() {
    let fmt = AssFormat;
    let content =
        "[Script Info]\nScriptType: v4.00+\n\n[V4+ Styles]\nFormat: Name\nStyle: Default\n";
    let err = fmt.parse(content).expect_err("missing [Events] must error");
    assert!(
        err.to_string().contains("[Events]"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn test_parse_ass_bom_valid_content_is_accepted() {
    let fmt = AssFormat;
    // Prepend a UTF-8 BOM ahead of an otherwise valid ASS payload.
    let mut content = String::from("\u{FEFF}");
    content.push_str(SAMPLE_ASS);
    let subtitle = fmt
        .parse(&content)
        .expect("BOM-prefixed valid ASS must parse");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Hello\nASS");
}

#[test]
fn test_parse_ass_bom_invalid_content_rejected() {
    let fmt = AssFormat;
    // BOM followed by content that has no [Events] section.
    let content = "\u{FEFF}This is not a valid ASS file";
    let err = fmt
        .parse(content)
        .expect_err("BOM-prefixed garbage must error");
    assert!(matches!(
        err,
        crate::error::SubXError::SubtitleFormat { .. }
    ));
}

#[test]
fn test_parse_ass_negative_timestamp_skipped() {
    let fmt = AssFormat;
    // First Dialogue has a negative start; second is well-formed.
    let content = "[Events]\n\
        Format: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n\
        Dialogue: 0,-0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,bad\n\
        Dialogue: 0,0:00:03.00,0:00:04.00,Default,,0000,0000,0000,,good\n";
    let subtitle = fmt
        .parse(content)
        .expect("negative timestamp must skip-and-continue");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "good");
}

#[test]
fn test_parse_ass_column_count_mismatch_skipped() {
    let fmt = AssFormat;
    // Format declares 10 fields; the first Dialogue supplies only 5 — skip.
    // The second Dialogue is well-formed and must still be parsed.
    let content = "[Events]\n\
        Format: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n\
        Dialogue: 0,0:00:01.00,0:00:02.50,Default\n\
        Dialogue: 0,0:00:03.00,0:00:04.00,Default,,0000,0000,0000,,good\n";
    let subtitle = fmt
        .parse(content)
        .expect("column-count mismatch must skip-and-continue");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "good");
}

#[test]
fn test_parse_ass_cue_below_size_cap_succeeds() {
    let fmt = AssFormat;
    // Build a Dialogue text just under the 1 MiB cap.
    let big = "x".repeat(MAX_CUE_BYTES - 16);
    let content = format!(
        "[Events]\n\
         Format: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n\
         Dialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,{}\n",
        big
    );
    let subtitle = fmt.parse(&content).expect("just-under-cap cue must parse");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text.len(), big.len());
}

#[test]
fn test_parse_ass_cue_above_size_cap_rejected() {
    let fmt = AssFormat;
    let big = "y".repeat(MAX_CUE_BYTES + 1);
    let content = format!(
        "[Events]\n\
         Format: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n\
         Dialogue: 0,0:00:01.00,0:00:02.50,Default,,0000,0000,0000,,{}\n",
        big
    );
    let err = fmt
        .parse(&content)
        .expect_err("over-cap cue must return SubtitleFormat error");
    assert!(matches!(
        err,
        crate::error::SubXError::SubtitleFormat { .. }
    ));
    assert_eq!(MAX_CUE_BYTES, 1024 * 1024);
}

#[test]
fn test_parser_module_exposes_max_cue_bytes_constant() {
    // Sanity check: the constant must remain at 1 MiB so the spec's
    // documented cap stays accurate.
    assert_eq!(parser::MAX_CUE_BYTES, 1 * 1024 * 1024);
}

#[cfg(feature = "slow-tests")]
mod proptests {
    //! Property-style mutation harness gated behind the `slow-tests`
    //! cargo feature. Asserts the ASS parser never panics for arbitrary
    //! byte input or for structurally-mutated golden fixtures.

    use super::AssFormat;
    use crate::core::formats::SubtitleFormat;
    use crate::core::formats::tests_support::{
        Rng, duplicate_random_line, flip_byte, inject_bom, oversize_cue, random_bytes,
        read_fixture, truncate,
    };

    const SEED: u64 = 0xDEAD_BEEF_0000_4153;
    const ITERATIONS: u64 = 200;
    const FIXTURES: &[&str] = &["ass/basic.ass", "ass/basic.crlf.ass", "ass/bom.ass"];

    fn drive(bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        let _ = AssFormat.parse(&s);
    }

    #[test]
    fn proptest_random_bytes_do_not_panic() {
        let mut rng = Rng::seeded(SEED);
        for _ in 0..ITERATIONS {
            let len = rng.gen_range(0, 4097) as usize;
            let buf = random_bytes(len, &mut rng);
            drive(&buf);
        }
    }

    #[test]
    fn proptest_mutated_fixtures_do_not_panic() {
        let mut rng = Rng::seeded(SEED ^ 0xA5A5_A5A5_A5A5_A5A5);
        let fixtures: Vec<Vec<u8>> = FIXTURES.iter().map(|p| read_fixture(p)).collect();
        for _ in 0..ITERATIONS {
            let base = &fixtures[(rng.next_u64() as usize) % fixtures.len()];
            let mutated = match rng.next_u64() % 5 {
                0 => flip_byte(base, &mut rng),
                1 => truncate(base, &mut rng),
                2 => duplicate_random_line(base, &mut rng),
                3 => inject_bom(base),
                _ => oversize_cue(base, &mut rng),
            };
            drive(&mutated);
        }
    }
}
