//! Unit tests for the SRT parser and serializer.

use super::SrtFormat;
use super::parser::MAX_CUE_BYTES_FOR_TESTS;
use crate::core::formats::{SubtitleFormat, SubtitleFormatType};
use crate::error::SubXError;
use std::time::Duration;

// NOTE: The following test data contains Chinese text for multi-line
// subtitle testing. This is allowed and does not require modification.
const SAMPLE_SRT: &str = "1\n00:00:01,000 --> 00:00:03,000\nHello, World!\n\n2\n00:00:05,000 --> 00:00:08,000\nThis is a test subtitle.\n多行測試\n\n";

#[test]
fn test_srt_parsing_basic() {
    let format = SrtFormat;
    let subtitle = format.parse(SAMPLE_SRT).unwrap();

    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.format, SubtitleFormatType::Srt);

    let first = &subtitle.entries[0];
    assert_eq!(first.index, 1);
    assert_eq!(first.start_time, Duration::from_millis(1000));
    assert_eq!(first.end_time, Duration::from_millis(3000));
    assert_eq!(first.text, "Hello, World!");

    let second = &subtitle.entries[1];
    assert_eq!(second.index, 2);
    assert_eq!(second.start_time, Duration::from_millis(5000));
    assert_eq!(second.end_time, Duration::from_millis(8000));
    assert_eq!(second.text, "This is a test subtitle.\n多行測試");
}

#[test]
fn test_srt_serialization_roundtrip() {
    let format = SrtFormat;
    let subtitle = format.parse(SAMPLE_SRT).unwrap();
    let serialized = format.serialize(&subtitle).unwrap();
    let reparsed = format.parse(&serialized).unwrap();
    assert_eq!(subtitle.entries.len(), reparsed.entries.len());
    for (o, r) in subtitle.entries.iter().zip(reparsed.entries.iter()) {
        assert_eq!(o.start_time, r.start_time);
        assert_eq!(o.end_time, r.end_time);
        assert_eq!(o.text, r.text);
    }
}

#[test]
fn test_srt_detection() {
    let format = SrtFormat;
    assert!(format.detect(SAMPLE_SRT));
    assert!(!format.detect("This is not SRT content"));
    assert!(!format.detect("WEBVTT\n\n00:00:01.000 --> 00:00:03.000\nHello"));
}

#[test]
fn test_srt_invalid_format() {
    let format = SrtFormat;
    let invalid_time = "1\n00:00:01 --> 00:00:03\nText\n\n";
    let subtitle = format.parse(invalid_time).unwrap();
    assert_eq!(subtitle.entries.len(), 0);
    let invalid_index = "invalid\n00:00:01,000 --> 00:00:03,000\nText\n\n";
    let subtitle = format.parse(invalid_index).unwrap();
    assert_eq!(subtitle.entries.len(), 0);
}

#[test]
fn test_srt_whitespace_only_input_returns_empty() {
    // `"\n\n\n"` is non-empty bytes but contains no cue — current behavior
    // is to return an empty `Subtitle`; only strictly empty input is
    // rejected (see `test_srt_empty_input_is_rejected`).
    let format = SrtFormat;
    let subtitle = format.parse("\n\n\n").unwrap();
    assert_eq!(subtitle.entries.len(), 0);

    let malformed = "1\n00:00:01,000 --> 00:00:03,000\n\n";
    let subtitle = format.parse(malformed).unwrap();
    assert_eq!(subtitle.entries.len(), 0);
}

#[test]
fn test_time_parsing_edge_cases() {
    let format = SrtFormat;
    let edge = "1\n23:59:59,999 --> 23:59:59,999\nEnd of day\n\n";
    let subtitle = format.parse(edge).unwrap();
    assert_eq!(subtitle.entries.len(), 1);
    let entry = &subtitle.entries[0];
    let expected = Duration::from_millis(23 * 3600000 + 59 * 60000 + 59 * 1000 + 999);
    assert_eq!(entry.start_time, expected);
    assert_eq!(entry.end_time, expected);
}

#[test]
fn test_file_extensions_and_name() {
    let format = SrtFormat;
    assert_eq!(format.file_extensions(), &["srt"]);
    assert_eq!(format.format_name(), "SRT");
}

#[test]
fn test_srt_bad_block_index_skipped() {
    let format = SrtFormat;
    let input = "notanumber\n00:00:01,000 --> 00:00:02,000\nBad block\n\n\
                 2\n00:00:03,000 --> 00:00:04,000\nGood block\n\n";
    let subtitle = format
        .parse(input)
        .expect("parser must not abort on bad block index");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].index, 2);
    assert_eq!(subtitle.entries[0].text, "Good block");
}

// ---------------------------------------------------------------------------
// Hardening tests (subtitle-parser-hardening matrix — SRT slice).
// ---------------------------------------------------------------------------

/// 5.1 — Empty input MUST return `SubXError::SubtitleFormat`.
#[test]
fn test_srt_empty_input_is_rejected() {
    let format = SrtFormat;
    let err = format.parse("").expect_err("empty input must be rejected");
    assert!(
        matches!(err, SubXError::SubtitleFormat { .. }),
        "expected SubtitleFormat error, got: {err:?}"
    );
}

/// 5.3 — Parser-level BOM consumption: BOM + valid content parses.
#[test]
fn test_srt_bom_prefixed_valid_content_parses() {
    let format = SrtFormat;
    let with_bom = format!("\u{FEFF}{}", SAMPLE_SRT);
    let subtitle = format
        .parse(&with_bom)
        .expect("BOM-prefixed valid SRT must parse");
    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.entries[0].text, "Hello, World!");
}

/// 5.3 — BOM with no content after stripping reduces to empty input → error.
#[test]
fn test_srt_bom_only_input_is_rejected() {
    let format = SrtFormat;
    let err = format
        .parse("\u{FEFF}")
        .expect_err("BOM-only input must be rejected as empty");
    assert!(matches!(err, SubXError::SubtitleFormat { .. }));
}

/// 5.4 — Out-of-order cues by start time MUST be preserved verbatim.
#[test]
fn test_srt_out_of_order_cues_are_preserved() {
    let format = SrtFormat;
    let input = "\
1\n00:00:10,000 --> 00:00:12,000\nLate\n\n\
2\n00:00:01,000 --> 00:00:03,000\nEarly\n\n";
    let subtitle = format.parse(input).expect("parse out-of-order");
    assert_eq!(subtitle.entries.len(), 2);
    assert_eq!(subtitle.entries[0].text, "Late");
    assert_eq!(
        subtitle.entries[0].start_time,
        Duration::from_millis(10_000)
    );
    assert_eq!(subtitle.entries[1].text, "Early");
    assert_eq!(subtitle.entries[1].start_time, Duration::from_millis(1_000));
}

/// 5.5 — Negative timestamp on a single block is skipped, the rest of the
/// file continues parsing.
#[test]
fn test_srt_negative_timestamp_is_skipped() {
    let format = SrtFormat;
    let input = "\
1\n-00:00:01,000 --> 00:00:03,000\nBad negative\n\n\
2\n00:00:05,000 --> 00:00:08,000\nGood\n\n";
    let subtitle = format
        .parse(input)
        .expect("parser must skip-and-continue on negative timestamps");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text, "Good");
}

/// 5.6 — A block just under the 1 MiB cap parses successfully.
#[test]
fn test_srt_cue_just_under_cap_parses() {
    let format = SrtFormat;
    let header = "1\n00:00:01,000 --> 00:00:03,000\n";
    let trailer = "\n\n";
    // Total block size = header + body + (trailing single `\n` of last
    // text line, which `block.len()` does NOT include because the block
    // is split on `\n\n`). Aim for header + body just under cap.
    let body_len = MAX_CUE_BYTES_FOR_TESTS - header.len() - 1;
    let body = "a".repeat(body_len);
    let input = format!("{}{}{}", header, body, trailer);

    let subtitle = format
        .parse(&input)
        .expect("cue just under the 1 MiB cap must parse");
    assert_eq!(subtitle.entries.len(), 1);
    assert_eq!(subtitle.entries[0].text.len(), body_len);
}

/// 5.6 — A block that exceeds the cap returns `SubXError::SubtitleFormat`.
#[test]
fn test_srt_cue_over_cap_is_rejected() {
    let format = SrtFormat;
    let header = "1\n00:00:01,000 --> 00:00:03,000\n";
    let trailer = "\n\n";
    // Make the block clearly over the cap.
    let body_len = MAX_CUE_BYTES_FOR_TESTS + 10;
    let body = "a".repeat(body_len);
    let input = format!("{}{}{}", header, body, trailer);

    let err = format
        .parse(&input)
        .expect_err("cue over the 1 MiB cap must be rejected");
    assert!(
        matches!(err, SubXError::SubtitleFormat { .. }),
        "expected SubtitleFormat error, got: {err:?}"
    );
}

// ── CRLF / line-ending tolerance regression tests ───────────────────────────

const SAMPLE_LF_3CUE: &str = concat!(
    "1\n00:00:01,000 --> 00:00:02,000\nHello\n\n",
    "2\n00:00:02,000 --> 00:00:03,000\nWorld\n\n",
    "3\n00:00:03,000 --> 00:00:04,000\nThree\n",
);

#[test]
fn srt_crlf_only_input_parses_all_cues() {
    let crlf = SAMPLE_LF_3CUE.replace('\n', "\r\n");
    let lf = SrtFormat.parse(SAMPLE_LF_3CUE).unwrap();
    let parsed = SrtFormat.parse(&crlf).unwrap();
    assert_eq!(parsed.entries.len(), lf.entries.len());
    assert_eq!(parsed.entries.len(), 3);
    for (a, b) in parsed.entries.iter().zip(lf.entries.iter()) {
        assert_eq!(a.index, b.index);
        assert_eq!(a.start_time, b.start_time);
        assert_eq!(a.end_time, b.end_time);
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn srt_mixed_lf_and_crlf_parses_correctly() {
    // First cue uses CRLF, second uses LF, with one CRLF+LF (`\r\n\n`)
    // separator between the second and third blocks.
    let mixed = concat!(
        "1\r\n00:00:01,000 --> 00:00:02,000\r\nHello\r\n\r\n",
        "2\n00:00:02,000 --> 00:00:03,000\nWorld\r\n\n",
        "3\n00:00:03,000 --> 00:00:04,000\nThree\n",
    );
    let parsed = SrtFormat.parse(mixed).unwrap();
    let lf = SrtFormat.parse(SAMPLE_LF_3CUE).unwrap();
    assert_eq!(parsed.entries.len(), lf.entries.len());
    for (a, b) in parsed.entries.iter().zip(lf.entries.iter()) {
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn srt_bare_cr_blank_line_separates_blocks() {
    // Old-Mac convention: bare `\r` line terminator with `\r\r`
    // separating cue blocks.
    let bare = SAMPLE_LF_3CUE.replace('\n', "\r");
    let parsed = SrtFormat.parse(&bare).unwrap();
    let lf = SrtFormat.parse(SAMPLE_LF_3CUE).unwrap();
    assert_eq!(parsed.entries.len(), 3);
    for (a, b) in parsed.entries.iter().zip(lf.entries.iter()) {
        assert_eq!(a.text, b.text);
    }
}

#[test]
fn srt_multi_line_cue_text_with_crlf_preserves_text() {
    let crlf = "1\r\n00:00:01,000 --> 00:00:02,000\r\nLine1\r\nLine2\r\n";
    let lf = "1\n00:00:01,000 --> 00:00:02,000\nLine1\nLine2\n";
    let parsed_crlf = SrtFormat.parse(crlf).unwrap();
    let parsed_lf = SrtFormat.parse(lf).unwrap();
    assert_eq!(parsed_crlf.entries.len(), 1);
    assert_eq!(parsed_crlf.entries[0].text, parsed_lf.entries[0].text);
    assert_eq!(parsed_crlf.entries[0].text, "Line1\nLine2");
}

#[test]
fn srt_crlf_input_does_not_collapse_into_single_cue() {
    // Regression guard for the original bug: a 2-cue CRLF SRT used to
    // parse to a single entry whose text payload absorbed the rest of
    // the file (including the `-->` arrow).
    let crlf = concat!(
        "1\r\n00:00:01,000 --> 00:00:02,000\r\nFirst\r\n\r\n",
        "2\r\n00:00:02,000 --> 00:00:03,000\r\nSecond\r\n",
    );
    let parsed = SrtFormat.parse(crlf).unwrap();
    assert!(
        parsed.entries.len() > 1,
        "CRLF input collapsed into one cue"
    );
    assert!(
        !parsed.entries[0].text.contains("-->"),
        "first cue's text payload absorbed a timing line: {:?}",
        parsed.entries[0].text
    );
}

#[test]
fn srt_crlf_oversized_cue_caps_on_raw_bytes() {
    // Build a single cue whose RAW byte length exceeds 1 MiB but whose
    // NORMALIZED length stays under 1 MiB by using `\r\n` line
    // terminators (each `\r\n` collapses to `\n`, shrinking the
    // payload by 1 byte per line).
    let header = "1\r\n00:00:01,000 --> 00:00:02,000\r\n";
    // Each payload line is `x\r\n` (3 raw bytes, 2 normalized bytes).
    // Choose N so raw > 1 MiB but normalized < 1 MiB.
    let line_count: usize = 400_000;
    let mut payload = String::with_capacity(line_count * 3);
    for _ in 0..line_count {
        payload.push_str("x\r\n");
    }
    let oversized = format!("{header}{payload}");
    assert!(
        oversized.len() > MAX_CUE_BYTES_FOR_TESTS,
        "test setup: raw must exceed cap"
    );
    // Lock Decision 7: normalized length must be under the cap, so the
    // only way the parser can reject this input is by checking the raw
    // pre-normalization bytes.
    let normalized_len = oversized.replace("\r\n", "\n").replace('\r', "\n").len();
    assert!(
        normalized_len <= MAX_CUE_BYTES_FOR_TESTS,
        "test setup: normalized must fit under cap to prove raw-byte enforcement"
    );
    let err = SrtFormat.parse(&oversized).unwrap_err();
    assert!(
        matches!(err, SubXError::SubtitleFormat { .. }),
        "expected SubtitleFormat error for raw-oversized CRLF cue, got: {err:?}"
    );
}

#[test]
fn srt_crlf_with_malformed_block_skips_and_continues() {
    // Middle block has a non-numeric index — should be skipped while
    // the surrounding cues parse normally.
    let crlf = concat!(
        "1\r\n00:00:01,000 --> 00:00:02,000\r\nFirst\r\n\r\n",
        "notanumber\r\n00:00:02,000 --> 00:00:03,000\r\nSkipped\r\n\r\n",
        "3\r\n00:00:03,000 --> 00:00:04,000\r\nThird\r\n",
    );
    let parsed = SrtFormat.parse(crlf).unwrap();
    assert_eq!(parsed.entries.len(), 2);
    assert_eq!(parsed.entries[0].text, "First");
    assert_eq!(parsed.entries[1].text, "Third");
}

#[cfg(feature = "slow-tests")]
mod proptests {
    //! Property-style mutation harness gated behind the `slow-tests`
    //! cargo feature. Asserts the SRT parser never panics for arbitrary
    //! byte input or for structurally-mutated golden fixtures.

    use super::SrtFormat;
    use crate::core::formats::SubtitleFormat;
    use crate::core::formats::tests_support::{
        Rng, duplicate_random_line, flip_byte, inject_bom, oversize_cue, random_bytes,
        read_fixture, truncate,
    };

    const SEED: u64 = 0xDEAD_BEEF_0000_5254;
    const ITERATIONS: u64 = 200;
    const FIXTURES: &[&str] = &["srt/basic.srt", "srt/basic.crlf.srt", "srt/bom.srt"];

    fn drive(bytes: &[u8]) {
        let s = String::from_utf8_lossy(bytes);
        let _ = SrtFormat.parse(&s);
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
