//! Line-ending normalization helpers shared by the SRT and VTT
//! parsers.
//!
//! The parsers split cue blocks on blank lines, but historically only
//! recognized LF (`\n\n`) blank-line separators — leading to
//! mis-parses of CRLF (Windows) and bare-CR (legacy Mac) inputs. This
//! module centralizes the small pieces of CR/LF logic both parsers
//! need:
//!
//! - [`normalize_line_endings`] performs an allocation-free fast path
//!   for LF-only inputs and otherwise collapses every CRLF sequence
//!   and every bare CR to a single LF.
//! - [`RawBlocks`] / [`raw_blocks`] walk the *raw* (pre-normalization)
//!   buffer and yield each cue-block byte slice, splitting on any
//!   blank-line separator regardless of the line-ending convention
//!   (`\n\n`, `\r\n\r\n`, `\r\n\n`, `\n\r\n`, `\r\r`, …). This is
//!   used to enforce the 1 MiB per-cue cap on raw bytes so the limit
//!   cannot be bypassed by stuffing in `\r` characters that disappear
//!   after normalization.

use std::borrow::Cow;

/// Collapse CRLF and bare CR line terminators in `content` to LF.
///
/// Returns a borrowed slice when the input contains no `\r` byte
/// (the dominant case), otherwise allocates a new buffer with every
/// `\r\n` and every remaining bare `\r` replaced by `\n`.
pub(crate) fn normalize_line_endings(content: &str) -> Cow<'_, str> {
    if !content.contains('\r') {
        Cow::Borrowed(content)
    } else {
        Cow::Owned(content.replace("\r\n", "\n").replace('\r', "\n"))
    }
}

/// Iterator returned by [`raw_blocks`].
pub(crate) struct RawBlocks<'a> {
    src: &'a str,
    pos: usize,
    done: bool,
}

/// Walk the *raw* (pre-normalization) `content` and yield each cue
/// block as a byte-slice of the original input, splitting on any
/// blank-line separator regardless of line-ending convention.
///
/// A blank-line separator is any pair of consecutive line terminators
/// (`\r\n`, `\n`, or `\r`); the iterator transparently consumes all
/// of `\n\n`, `\r\n\r\n`, `\r\n\n`, `\n\r\n`, `\r\r`, …
///
/// The number of yielded blocks equals
/// `normalize_line_endings(content).split("\n\n").count()`, so the
/// blocks line up one-to-one with the post-normalization parser flow.
pub(crate) fn raw_blocks(content: &str) -> RawBlocks<'_> {
    RawBlocks {
        src: content,
        pos: 0,
        done: false,
    }
}

impl<'a> Iterator for RawBlocks<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.done {
            return None;
        }
        let bytes = self.src.as_bytes();
        let start = self.pos;
        let mut i = start;
        while i < bytes.len() {
            let first_term = term_len_at(bytes, i);
            if first_term == 0 {
                i += 1;
                continue;
            }
            let after_first = i + first_term;
            let second_term = if after_first < bytes.len() {
                term_len_at(bytes, after_first)
            } else {
                0
            };
            if second_term > 0 {
                // Blank-line separator found. Block bytes are
                // `[start, i)` (exclusive of the first terminator).
                // CR and LF are ASCII so the slice falls on a UTF-8
                // boundary.
                let block = &self.src[start..i];
                self.pos = after_first + second_term;
                return Some(block);
            }
            i = after_first;
        }
        // No more separators; emit the trailing block (which may be
        // empty if the buffer ended on an odd terminator) and stop.
        self.done = true;
        Some(&self.src[start..])
    }
}

/// Length of the line terminator at byte offset `i`, or 0 if the byte
/// is not the start of one. Recognizes `\r\n`, `\n`, and bare `\r`.
fn term_len_at(bytes: &[u8], i: usize) -> usize {
    match bytes[i] {
        b'\r' => {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                2
            } else {
                1
            }
        }
        b'\n' => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_lf_only_is_borrowed() {
        let s = "abc\ndef\n\nghi";
        let out = normalize_line_endings(s);
        assert!(matches!(out, Cow::Borrowed(_)));
        assert_eq!(out, s);
    }

    #[test]
    fn normalize_crlf_collapses_to_lf() {
        let s = "abc\r\ndef\r\n\r\nghi";
        let out = normalize_line_endings(s);
        assert_eq!(out, "abc\ndef\n\nghi");
    }

    #[test]
    fn normalize_bare_cr_collapses_to_lf() {
        let s = "abc\rdef\r\rghi";
        let out = normalize_line_endings(s);
        assert_eq!(out, "abc\ndef\n\nghi");
    }

    #[test]
    fn normalize_mixed_line_endings() {
        let s = "abc\r\ndef\nghi\r\rjkl\r\n\nmno";
        let out = normalize_line_endings(s);
        assert_eq!(out, "abc\ndef\nghi\n\njkl\n\nmno");
    }

    #[test]
    fn raw_blocks_lf_only() {
        let s = "a\nb\n\nc\nd";
        let v: Vec<&str> = raw_blocks(s).collect();
        assert_eq!(v, vec!["a\nb", "c\nd"]);
    }

    #[test]
    fn raw_blocks_crlf() {
        let s = "a\r\nb\r\n\r\nc\r\nd";
        let v: Vec<&str> = raw_blocks(s).collect();
        assert_eq!(v, vec!["a\r\nb", "c\r\nd"]);
    }

    #[test]
    fn raw_blocks_bare_cr() {
        let s = "a\rb\r\rc\rd";
        let v: Vec<&str> = raw_blocks(s).collect();
        assert_eq!(v, vec!["a\rb", "c\rd"]);
    }

    #[test]
    fn raw_blocks_mixed_separators() {
        // \r\n\n and \n\r\n should both act as blank-line separators.
        let s = "a\r\nb\r\n\nc\r\nd\n\r\ne";
        let v: Vec<&str> = raw_blocks(s).collect();
        assert_eq!(v, vec!["a\r\nb", "c\r\nd", "e"]);
    }

    #[test]
    fn raw_blocks_count_matches_normalized_split() {
        let inputs = [
            "a\nb\n\nc\nd",
            "a\r\nb\r\n\r\nc\r\nd",
            "a\rb\r\rc\rd",
            "a\r\nb\r\n\nc\r\nd\n\r\ne",
            "single block no separator",
            "",
        ];
        for s in inputs {
            let raw_count = raw_blocks(s).count();
            let normalized = normalize_line_endings(s);
            let norm_count = normalized.split("\n\n").count();
            assert_eq!(raw_count, norm_count, "input={s:?}");
        }
    }
}
