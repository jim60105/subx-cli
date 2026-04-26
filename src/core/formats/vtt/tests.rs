//! Unit tests for the WebVTT parser and serializer.

use super::VttFormat;
use super::parser::MAX_CUE_BYTES;
use crate::core::formats::{
    Subtitle, SubtitleEntry, SubtitleFormat, SubtitleFormatType, SubtitleMetadata,
};
use crate::error::SubXError;
use std::time::Duration;

const SAMPLE: &str = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:03.500\nHello\nWorld\n";

#[test]
fn test_parse_and_serialize() {
    let fmt = VttFormat;
    let subtitle = fmt.parse(SAMPLE).expect("parse failed");
    assert_eq!(subtitle.entries.len(), 1);
    let out = fmt.serialize(&subtitle).expect("serialize failed");
    assert!(out.contains("00:00:01.000 --> 00:00:03.500"));
}

#[test]
fn test_detect_and_skip_headers() {
    let fmt = VttFormat;
    assert!(fmt.detect("WEBVTT\nContent"));
    assert!(!fmt.detect("00:00:00.000 --> 00:00:01.000"));
}

#[test]
fn test_parse_with_note_and_style() {
    let content = "WEBVTT\n\nNOTE this is note\nSTYLE body {color:red}\n\n1\n00:00:02.000 --> 00:00:03.000\nTest\n";
    let fmt = VttFormat;
    let subtitle = fmt.parse(content).expect("parse with NOTE/STYLE failed");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Test");
}

#[test]
fn test_serialize_multiple_entries() {
    let mut subtitle = Subtitle {
        entries: Vec::new(),
        metadata: SubtitleMetadata {
            title: None,
            language: None,
            encoding: "utf-8".to_string(),
            frame_rate: None,
            original_format: SubtitleFormatType::Vtt,
        },
        format: SubtitleFormatType::Vtt,
    };
    subtitle.entries.push(SubtitleEntry {
        index: 1,
        start_time: Duration::from_secs(1),
        end_time: Duration::from_secs(2),
        text: "A".into(),
        styling: None,
    });
    subtitle.entries.push(SubtitleEntry {
        index: 2,
        start_time: Duration::from_secs(3),
        end_time: Duration::from_secs(4),
        text: "B".into(),
        styling: None,
    });
    let fmt = VttFormat;
    let out = fmt.serialize(&subtitle).expect("serialize multiple failed");
    assert!(out.contains("WEBVTT"));
    assert!(out.contains("1\n"));
    assert!(out.contains("2\n"));
}

// ---------------------------------------------------------------------
// Hardening matrix tests
// ---------------------------------------------------------------------

/// 5.1 — empty input is rejected with a typed error.
#[test]
fn test_empty_input_rejected() {
    let fmt = VttFormat;
    let err = fmt.parse("").expect_err("empty input must error");
    assert!(matches!(err, SubXError::SubtitleFormat { .. }));
}

/// 5.2 — content without a `WEBVTT` signature is rejected.
#[test]
fn test_missing_webvtt_header_rejected() {
    let fmt = VttFormat;
    let content = "1\n00:00:01.000 --> 00:00:02.000\nNo header\n";
    let err = fmt
        .parse(content)
        .expect_err("missing WEBVTT signature must error");
    assert!(matches!(err, SubXError::SubtitleFormat { .. }));
}

/// 5.3 — leading UTF-8 BOM with valid content parses successfully.
#[test]
fn test_bom_with_valid_content_parses() {
    let fmt = VttFormat;
    let content = format!("\u{feff}{}", SAMPLE);
    let subtitle = fmt.parse(&content).expect("BOM + valid content");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Hello\nWorld");
}

/// 5.3 — leading UTF-8 BOM with otherwise invalid content returns a typed error.
#[test]
fn test_bom_with_invalid_content_rejected() {
    let fmt = VttFormat;
    let content = "\u{feff}not a vtt file at all\n";
    let err = fmt.parse(content).expect_err("BOM + invalid must error");
    assert!(matches!(err, SubXError::SubtitleFormat { .. }));
}

/// 5.4 — out-of-order cues are preserved (no implicit sort).
#[test]
fn test_out_of_order_cues_preserved() {
    let fmt = VttFormat;
    let content = "WEBVTT\n\n\
        1\n00:00:10.000 --> 00:00:11.000\nLater\n\n\
        2\n00:00:01.000 --> 00:00:02.000\nEarlier\n";
    let subtitle = fmt.parse(content).expect("parse out-of-order");
    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.entries[0].text, "Later");
    assert_eq!(subtitle.entries[1].text, "Earlier");
    assert_eq!(subtitle.entries[0].start_time, Duration::from_secs(10));
    assert_eq!(subtitle.entries[1].start_time, Duration::from_secs(1));
}

/// 5.5 — a cue with a negative-timestamp marker is skipped, parsing
/// continues with subsequent valid cues.
#[test]
fn test_negative_timestamp_skipped() {
    let fmt = VttFormat;
    let content = "WEBVTT\n\n\
        1\n-00:00:01.000 --> 00:00:02.000\nNegative cue\n\n\
        2\n00:00:03.000 --> 00:00:04.000\nValid cue\n";
    let subtitle = fmt.parse(content).expect("parse with negative cue");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Valid cue");
}

/// 5.6 — a cue body just under the per-cue cap parses successfully.
#[test]
fn test_cue_just_under_cap_succeeds() {
    let fmt = VttFormat;
    let body_len = MAX_CUE_BYTES - 4096;
    let body: String = "a".repeat(body_len);
    let content = format!("WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.000\n{}\n", body);
    let subtitle = fmt.parse(&content).expect("just-under cap must succeed");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text.len(), body_len);
}

/// 5.6 — a cue body just over the per-cue cap is rejected.
#[test]
fn test_cue_just_over_cap_rejected() {
    let fmt = VttFormat;
    let body: String = "a".repeat(MAX_CUE_BYTES + 1);
    let content = format!("WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.000\n{}\n", body);
    let err = fmt.parse(&content).expect_err("over-cap cue must error");
    assert!(matches!(err, SubXError::SubtitleFormat { .. }));
}

/// 5.8 — VTT files missing a trailing blank line at EOF still recognize
/// the final cue.
#[test]
fn test_no_trailing_blank_line_at_eof() {
    let fmt = VttFormat;
    let content = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.000\nFinal cue";
    let subtitle = fmt.parse(content).expect("parse without trailing blank");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Final cue");
}

#[cfg(feature = "slow-tests")]
mod proptests {
    //! Property-style mutation harness gated behind the `slow-tests`
    //! cargo feature. Asserts the VTT parser never panics for arbitrary
    //! byte input or for structurally-mutated golden fixtures.

    use super::VttFormat;
    use crate::core::formats::SubtitleFormat;
    use crate::core::formats::tests_support::{
        Rng, duplicate_random_line, flip_byte, inject_bom, oversize_cue, random_bytes,
        read_fixture, truncate,
    };

    const SEED: u64 = 0xDEAD_BEEF_0000_5654;
    const ITERATIONS: u64 = 200;
    const FIXTURES: &[&str] = &["vtt/basic.vtt", "vtt/basic.crlf.vtt", "vtt/bom.vtt"];

    fn drive(bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        let _ = VttFormat.parse(&s);
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
