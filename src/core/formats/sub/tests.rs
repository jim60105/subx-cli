//! Unit tests for the SUB format adapter.

use super::SubFormat;
use super::parser::MAX_CUE_BYTES;
use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use std::time::Duration;

const SAMPLE: &str = "{10}{20}Hello|World\n";

#[test]
fn test_parse_and_serialize() {
    let fmt = SubFormat;
    let subtitle = fmt.parse(SAMPLE).expect("parse failed");
    assert_eq!(subtitle.entries.len(), 1);
    let out = fmt.serialize(&subtitle).expect("serialize failed");
    assert!(out.contains("{10}{20}Hello|World"));
}

#[test]
fn test_detect_true_and_false() {
    let fmt = SubFormat;
    assert!(fmt.detect(SAMPLE));
    assert!(!fmt.detect("random text"));
}

#[test]
fn test_parse_multiple_and_frame_rate() {
    let custom = "{0}{25}First|Line\n{25}{50}Second|Line\n";
    let fmt = SubFormat;
    let subtitle = fmt.parse(custom).expect("parse multiple failed");
    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.metadata.frame_rate, Some(25.0));
    assert_eq!(subtitle.entries[0].text, "First\nLine");
    assert_eq!(subtitle.entries[1].text, "Second\nLine");
}

#[test]
fn test_serialize_with_nondefault_fps() {
    let mut subtitle = Subtitle {
        entries: Vec::new(),
        metadata: SubtitleMetadata {
            title: None,
            language: None,
            encoding: "utf-8".to_string(),
            frame_rate: Some(50.0),
            original_format: SubtitleFormatType::Sub,
        },
        format: SubtitleFormatType::Sub,
    };
    subtitle.entries.push(SubtitleEntry {
        index: 1,
        start_time: Duration::from_secs_f64(1.0),
        end_time: Duration::from_secs_f64(2.0),
        text: "X".into(),
        styling: None,
    });
    let fmt = SubFormat;
    let out = fmt.serialize(&subtitle).expect("serialize fps failed");
    // 1s * 50fps = 50 frames
    assert!(out.contains("{50}{100}X"));
}

#[test]
fn test_parse_sub_skips_huge_frame_numbers() {
    // 25fps -> 86_400_000ms (24h) corresponds to 2_160_000 frames.
    // Use a frame count well above that so the entry must be skipped.
    let content = "{0}{25}Good\n{999999999}{999999999}TooBig\n{50}{75}AlsoGood\n";
    let fmt = SubFormat;
    let subtitle = fmt.parse(content).expect("parse must succeed");
    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.entries[0].text, "Good");
    assert_eq!(subtitle.entries[1].text, "AlsoGood");
}

// ---------------------------------------------------------------------------
// Hardening tests (subtitle-parser-hardening matrix)
// ---------------------------------------------------------------------------

#[test]
fn test_parse_empty_input_returns_typed_error() {
    let fmt = SubFormat;
    let err = fmt.parse("").expect_err("empty input must error");
    assert!(
        matches!(err, SubXError::SubtitleFormat { .. }),
        "expected SubtitleFormat error, got: {err:?}"
    );
}

#[test]
fn test_parse_skips_non_numeric_frame_range_and_continues() {
    // The middle line is not a valid `{digits}{digits}text` cue. The
    // parser must skip it (debug-log) and emit the surrounding good cues.
    let content = "{0}{25}Good\n{abc}{def}Garbage\nplain text line\n{50}{75}AlsoGood\n";
    let fmt = SubFormat;
    let subtitle = fmt.parse(content).expect("parse must succeed");
    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.entries[0].text, "Good");
    assert_eq!(subtitle.entries[1].text, "AlsoGood");
}

#[test]
fn test_parse_per_cue_byte_cap_just_under_succeeds() {
    // Cue body exactly at the cap is allowed.
    let body = "a".repeat(MAX_CUE_BYTES);
    let content = format!("{{0}}{{25}}{}\n", body);
    let fmt = SubFormat;
    let subtitle = fmt
        .parse(&content)
        .expect("cue exactly at MAX_CUE_BYTES must parse");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text.len(), MAX_CUE_BYTES);
}

#[test]
fn test_parse_per_cue_byte_cap_just_over_errors() {
    // Cue body one byte over the cap must trigger a typed error.
    let body = "a".repeat(MAX_CUE_BYTES + 1);
    let content = format!("{{0}}{{25}}{}\n", body);
    let fmt = SubFormat;
    let err = fmt
        .parse(&content)
        .expect_err("cue over MAX_CUE_BYTES must error");
    assert!(
        matches!(err, SubXError::SubtitleFormat { .. }),
        "expected SubtitleFormat error, got: {err:?}"
    );
}

#[cfg(feature = "slow-tests")]
mod proptests {
    //! Property-style mutation harness gated behind the `slow-tests`
    //! cargo feature. Asserts the SUB parser never panics for arbitrary
    //! byte input or for structurally-mutated golden fixtures.

    use super::SubFormat;
    use crate::core::formats::SubtitleFormat;
    use crate::core::formats::tests_support::{
        Rng, duplicate_random_line, flip_byte, inject_bom, oversize_cue, random_bytes,
        read_fixture, truncate,
    };

    const SEED: u64 = 0xDEAD_BEEF_0000_5542;
    const ITERATIONS: u64 = 200;
    const FIXTURES: &[&str] = &["sub/basic.sub", "sub/basic.crlf.sub"];

    fn drive(bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        let _ = SubFormat.parse(&s);
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
