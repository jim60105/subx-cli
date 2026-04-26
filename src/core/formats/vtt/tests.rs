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

// ── CRLF / line-ending tolerance regression tests ───────────────────────────

const SAMPLE_LF_3CUE_VTT: &str = concat!(
    "WEBVTT\n\n",
    "1\n00:00:01.000 --> 00:00:02.000\nHello\n\n",
    "2\n00:00:02.000 --> 00:00:03.000\nWorld\n\n",
    "3\n00:00:03.000 --> 00:00:04.000\nThree\n",
);

#[test]
fn vtt_crlf_only_input_parses_all_cues() {
    let crlf = SAMPLE_LF_3CUE_VTT.replace('\n', "\r\n");
    let lf = VttFormat.parse(SAMPLE_LF_3CUE_VTT).unwrap();
    let parsed = VttFormat.parse(&crlf).unwrap();
    assert_eq!(parsed.entries.len(), 3);
    assert_eq!(parsed.entries.len(), lf.entries.len());
    for (a, b) in parsed.entries.iter().zip(lf.entries.iter()) {
        assert_eq!(a.start_time, b.start_time);
        assert_eq!(a.end_time, b.end_time);
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn vtt_mixed_lf_and_crlf_parses_correctly() {
    // CRLF header + LF body + one `\r\n\n` separator inside the body.
    let mixed = concat!(
        "WEBVTT\r\n\r\n",
        "1\n00:00:01.000 --> 00:00:02.000\nHello\r\n\n",
        "2\n00:00:02.000 --> 00:00:03.000\nWorld\n\n",
        "3\r\n00:00:03.000 --> 00:00:04.000\r\nThree\r\n",
    );
    let parsed = VttFormat.parse(mixed).unwrap();
    let lf = VttFormat.parse(SAMPLE_LF_3CUE_VTT).unwrap();
    assert_eq!(parsed.entries.len(), lf.entries.len());
    for (a, b) in parsed.entries.iter().zip(lf.entries.iter()) {
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn vtt_bare_cr_blank_line_separates_blocks() {
    let bare = SAMPLE_LF_3CUE_VTT.replace('\n', "\r");
    let parsed = VttFormat.parse(&bare).unwrap();
    let lf = VttFormat.parse(SAMPLE_LF_3CUE_VTT).unwrap();
    assert_eq!(parsed.entries.len(), 3);
    for (a, b) in parsed.entries.iter().zip(lf.entries.iter()) {
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn vtt_multi_line_cue_text_with_crlf_preserves_text() {
    let crlf = "WEBVTT\r\n\r\n1\r\n00:00:01.000 --> 00:00:02.000\r\nLine1\r\nLine2\r\n";
    let lf = "WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.000\nLine1\nLine2\n";
    let parsed_crlf = VttFormat.parse(crlf).unwrap();
    let parsed_lf = VttFormat.parse(lf).unwrap();
    assert_eq!(parsed_crlf.entries.len(), 1);
    assert_eq!(parsed_crlf.entries[0].text, parsed_lf.entries[0].text);
    assert_eq!(parsed_crlf.entries[0].text, "Line1\nLine2");
}

#[test]
fn vtt_crlf_input_does_not_yield_zero_entries() {
    // Regression guard for the original bug: a 2-cue CRLF VTT used to
    // parse to zero entries because the cue marker line's trailing
    // `\r` defeated the timing regex.
    let crlf = concat!(
        "WEBVTT\r\n\r\n",
        "1\r\n00:00:01.000 --> 00:00:02.000\r\nFirst\r\n\r\n",
        "2\r\n00:00:02.000 --> 00:00:03.000\r\nSecond\r\n",
    );
    let parsed = VttFormat.parse(crlf).unwrap();
    assert_eq!(parsed.entries.len(), 2);
}

#[test]
fn vtt_crlf_oversized_cue_caps_on_raw_bytes() {
    let header = "WEBVTT\r\n\r\n1\r\n00:00:01.000 --> 00:00:02.000\r\n";
    let line_count: usize = 400_000;
    let mut payload = String::with_capacity(line_count * 3);
    for _ in 0..line_count {
        payload.push_str("x\r\n");
    }
    let oversized = format!("{header}{payload}");
    assert!(
        oversized.len() > MAX_CUE_BYTES,
        "test setup: raw must exceed cap"
    );
    // Lock Decision 7: normalized length must be under the cap, so the
    // only way the parser can reject this input is by checking the raw
    // pre-normalization bytes.
    let normalized_len = oversized.replace("\r\n", "\n").replace('\r', "\n").len();
    assert!(
        normalized_len <= MAX_CUE_BYTES,
        "test setup: normalized must fit under cap to prove raw-byte enforcement"
    );
    let err = VttFormat.parse(&oversized).unwrap_err();
    assert!(
        matches!(err, SubXError::SubtitleFormat { .. }),
        "expected SubtitleFormat error for raw-oversized CRLF cue, got: {err:?}"
    );
}

#[test]
fn vtt_oversized_note_block_is_skipped_not_rejected() {
    // A `NOTE` block whose raw byte length exceeds the per-cue cap
    // must still be skipped silently — the cap applies only to cue
    // blocks. Constructed via `NOTE\r\n` followed by many short lines
    // so the block stays a single block (no blank-line separators
    // inside it).
    let line_count: usize = 400_000;
    let mut note = String::with_capacity(8 + line_count * 3);
    note.push_str("NOTE\r\n");
    for _ in 0..line_count {
        note.push_str("x\r\n");
    }
    let input = format!("WEBVTT\r\n\r\n{note}\r\n1\r\n00:00:01.000 --> 00:00:02.000\r\nHello\r\n");
    assert!(
        input.len() > MAX_CUE_BYTES,
        "test setup: NOTE must exceed cap"
    );
    let parsed = VttFormat
        .parse(&input)
        .expect("oversized NOTE block must be skipped, not rejected");
    assert_eq!(parsed.entries.len(), 1);
    assert_eq!(parsed.entries[0].text, "Hello");
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
