//! Shared UUIDv7 identifier generation with strict 1ms spacing.
//!
//! This generator is used by both the subtitle translation engine (for cue
//! IDs) and the media discovery layer (for file IDs). Generating UUIDv7 IDs
//! in batch order means the embedded `unix_time_ts` (the most-significant 48
//! bits) reflects request order and can be inspected from logs without
//! consulting external state.
//!
//! This module enforces an additional constraint on top of the UUIDv7
//! algorithm: adjacent ID generations are spaced by at least 1 millisecond,
//! so each next ID's `unix_time_ts` is strictly greater than the previous
//! one. This avoids same-millisecond ambiguity that the standard UUIDv7
//! algorithm normally resolves through random or monotonic counter bits.

use std::thread;
use std::time::Duration;

use uuid::Uuid;

/// Stateful UUIDv7 ID generator shared by media discovery and translation.
///
/// Calling [`Uuidv7Generator::next_id`] sleeps for at least 1ms when the
/// previous call observed the same millisecond, guaranteeing strictly
/// increasing `unix_time_ts` values across all IDs produced by a single
/// generator instance.
#[derive(Debug, Default)]
pub struct Uuidv7Generator {
    last_unix_ts_ms: Option<u64>,
}

impl Uuidv7Generator {
    /// Create a fresh generator.
    pub fn new() -> Self {
        Self {
            last_unix_ts_ms: None,
        }
    }

    /// Generate the next UUIDv7 ID with strict 1ms spacing.
    ///
    /// The implementation extracts the embedded `unix_time_ts` from the
    /// generated UUIDv7. If the new timestamp is not strictly greater than
    /// the previous one, the thread sleeps long enough for the next call to
    /// land in a later millisecond, then retries.
    pub fn next_id(&mut self) -> Uuid {
        loop {
            let id = Uuid::now_v7();
            let ts = unix_time_ms(&id);
            match self.last_unix_ts_ms {
                Some(last) if ts <= last => {
                    let wait = last.saturating_sub(ts).saturating_add(1);
                    thread::sleep(Duration::from_millis(wait));
                    continue;
                }
                _ => {
                    self.last_unix_ts_ms = Some(ts);
                    return id;
                }
            }
        }
    }
}

/// Generate `count` UUIDv7 IDs in sequence with strict 1ms spacing.
///
/// Convenience wrapper around [`Uuidv7Generator`] for callers that just need
/// a vector of IDs.
pub fn generate_ids(count: usize) -> Vec<Uuid> {
    let mut id_gen = Uuidv7Generator::new();
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(id_gen.next_id());
    }
    ids
}

/// Extract the 48-bit `unix_time_ts` field from a UUIDv7 value.
///
/// Returns the timestamp in milliseconds since the Unix epoch.
pub fn unix_time_ms(id: &Uuid) -> u64 {
    let bytes = id.as_bytes();
    ((bytes[0] as u64) << 40)
        | ((bytes[1] as u64) << 32)
        | ((bytes[2] as u64) << 24)
        | ((bytes[3] as u64) << 16)
        | ((bytes[4] as u64) << 8)
        | (bytes[5] as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_uuidv7() {
        let ids = generate_ids(3);
        for id in &ids {
            assert_eq!(id.get_version_num(), 7, "expected UUIDv7, got {}", id);
        }
    }

    #[test]
    fn timestamps_strictly_increase() {
        let ids = generate_ids(5);
        let mut last = 0u64;
        for (i, id) in ids.iter().enumerate() {
            let ts = unix_time_ms(id);
            if i > 0 {
                assert!(
                    ts > last,
                    "expected strictly increasing unix_time_ts, got {} after {}",
                    ts,
                    last
                );
            }
            last = ts;
        }
    }

    #[test]
    fn tight_loop_strictly_monotonic_unix_time_ts() {
        // Regression: even when next_id is called as fast as possible, the
        // 1ms-spacing contract guarantees strictly increasing unix_time_ts.
        let mut generator = Uuidv7Generator::new();
        let mut last_ts = 0u64;
        for i in 0..20 {
            let id = generator.next_id();
            let ts = unix_time_ms(&id);
            if i > 0 {
                assert!(
                    ts > last_ts,
                    "expected strictly increasing unix_time_ts at iter {}, got {} after {}",
                    i,
                    ts,
                    last_ts
                );
            }
            last_ts = ts;
        }
    }
}
