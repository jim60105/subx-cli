//! Test-only deterministic mutation harness for the slow-tests
//! property/fuzz coverage layer.
//!
//! This module is intentionally self-contained and depends on `std` only.
//! It is gated behind `#[cfg(all(test, feature = "slow-tests"))]` so it
//! never participates in default builds and contributes nothing to
//! `Cargo.lock` resolution.
//!
//! See `openspec/changes/refactor-format-parsers/specs/subtitle-parser-hardening/spec.md`
//! ("Property-style parser fuzz coverage gated by feature") for the
//! contract this harness implements.

use std::path::PathBuf;

/// Deterministic xorshift64* pseudo-random number generator.
///
/// Suitable only for reproducible test inputs — not cryptographic use.
pub(crate) struct Rng {
    state: u64,
}

impl Rng {
    /// Construct a new generator seeded with `seed`. The seed is forced
    /// non-zero (xorshift64 degenerates on a zero state).
    pub(crate) fn seeded(seed: u64) -> Self {
        let s = if seed == 0 {
            0x9E37_79B9_7F4A_7C15
        } else {
            seed
        };
        Self { state: s }
    }

    /// Draw the next `u64`.
    pub(crate) fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Draw the next `u32`.
    pub(crate) fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Generate a value in the half-open range `[lo, hi)`.
    ///
    /// # Panics
    /// Panics if `lo >= hi`.
    pub(crate) fn gen_range(&mut self, lo: u64, hi: u64) -> u64 {
        assert!(lo < hi, "gen_range requires lo < hi");
        lo + (self.next_u64() % (hi - lo))
    }
}

/// Read a fixture from `tests/fixtures/formats/<rel_path>` as raw bytes.
///
/// `rel_path` is interpreted relative to the `tests/fixtures/formats/`
/// directory, e.g. `"srt/basic.srt"`.
pub(crate) fn read_fixture(rel_path: &str) -> Vec<u8> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set when running cargo tests");
    let mut p = PathBuf::from(manifest);
    p.push("tests");
    p.push("fixtures");
    p.push("formats");
    p.push(rel_path);
    std::fs::read(&p).unwrap_or_else(|e| panic!("failed to read fixture {}: {}", p.display(), e))
}

/// Flip a single random byte in `bytes` by XOR-ing it with a random mask.
pub(crate) fn flip_byte(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    let mut out = bytes.to_vec();
    if out.is_empty() {
        return out;
    }
    let idx = (rng.next_u64() as usize) % out.len();
    let mask = (rng.next_u32() as u8).max(1);
    out[idx] ^= mask;
    out
}

/// Truncate `bytes` to a random prefix length in `[0, bytes.len()]`.
pub(crate) fn truncate(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    let len = if bytes.is_empty() {
        0
    } else {
        (rng.next_u64() as usize) % (bytes.len() + 1)
    };
    bytes[..len].to_vec()
}

/// Find a random `\n`-delimited line and duplicate it in place.
///
/// If the input has no newlines, the entire buffer is duplicated.
pub(crate) fn duplicate_random_line(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    if bytes.is_empty() {
        return bytes.to_vec();
    }
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(
            bytes
                .iter()
                .enumerate()
                .filter_map(|(i, b)| if *b == b'\n' { Some(i + 1) } else { None }),
        )
        .filter(|s| *s <= bytes.len())
        .collect();
    if line_starts.is_empty() {
        let mut out = bytes.to_vec();
        out.extend_from_slice(bytes);
        return out;
    }
    let pick = (rng.next_u64() as usize) % line_starts.len();
    let start = line_starts[pick];
    let end = bytes[start..]
        .iter()
        .position(|b| *b == b'\n')
        .map(|p| start + p + 1)
        .unwrap_or(bytes.len());
    let mut out = Vec::with_capacity(bytes.len() + (end - start));
    out.extend_from_slice(&bytes[..end]);
    out.extend_from_slice(&bytes[start..end]);
    out.extend_from_slice(&bytes[end..]);
    out
}

/// Prepend a UTF-8 BOM (`EF BB BF`) to `bytes`.
pub(crate) fn inject_bom(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 3);
    out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    out.extend_from_slice(bytes);
    out
}

/// Inject an oversize cue body (`>= 1 MiB`) at a random newline boundary.
///
/// This is intended to drive parsers into the `MAX_CUE_BYTES`
/// rejection path. The injected payload is a contiguous run of `'a'`
/// bytes of length `1 MiB + 32` followed by a `\n` so it is interpreted
/// as a single oversized line/cue body by every parser.
pub(crate) fn oversize_cue(bytes: &[u8], rng: &mut Rng) -> Vec<u8> {
    const PAYLOAD_LEN: usize = 1024 * 1024 + 32;
    let payload: Vec<u8> = std::iter::repeat_n(b'a', PAYLOAD_LEN)
        .chain(std::iter::once(b'\n'))
        .collect();
    if bytes.is_empty() {
        return payload;
    }
    let insert_at = (rng.next_u64() as usize) % (bytes.len() + 1);
    let mut out = Vec::with_capacity(bytes.len() + payload.len());
    out.extend_from_slice(&bytes[..insert_at]);
    out.extend_from_slice(&payload);
    out.extend_from_slice(&bytes[insert_at..]);
    out
}

/// Generate a buffer of `len` deterministic random bytes.
pub(crate) fn random_bytes(len: usize, rng: &mut Rng) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        let chunk = rng.next_u64().to_le_bytes();
        let take = (len - out.len()).min(chunk.len());
        out.extend_from_slice(&chunk[..take]);
    }
    out
}
