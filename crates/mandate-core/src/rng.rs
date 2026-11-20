//! Deterministic RNG (DEC-041 rule 5).
//!
//! * **Sequential streams**: one ChaCha12 generator per subsystem, so adding a
//!   draw in one subsystem never shifts another subsystem's sequence.
//! * **Keyed access**: `keyed()` derives an order-independent generator from
//!   `(world seed, stream, key parts)` for lazily evaluated entities (DEC-018).
//!
//! Sampling helpers are implemented here (not via `rand` distributions) so a
//! dependency upgrade can never silently change simulation outcomes.

use crate::hash::{derive_seed, derive_u64};
use rand_chacha::ChaCha12Rng;
use rand_core::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum Stream {
    Worldgen,
    Population,
    Economy,
    Environment,
    Npc,
    Elections,
    Polling,
    Director,
    Dialogue,
    Player,
    Misc,
    /// Media, scandals, and the legal path (DEC-034, DEC-006): keyed draws only.
    Media,
    /// Asks and promises (DEC-097 §2): keyed draws only.
    Asks,
    /// Policy families (DEC-092): decks, bindings, works risks; keyed draws only.
    Policy,
}

impl Stream {
    pub const ALL: [Stream; 14] = [
        Stream::Worldgen,
        Stream::Population,
        Stream::Economy,
        Stream::Environment,
        Stream::Npc,
        Stream::Elections,
        Stream::Polling,
        Stream::Director,
        Stream::Dialogue,
        Stream::Player,
        Stream::Misc,
        Stream::Media,
        Stream::Asks,
        Stream::Policy,
    ];

    pub fn tag(self) -> &'static str {
        match self {
            Stream::Worldgen => "worldgen",
            Stream::Population => "population",
            Stream::Economy => "economy",
            Stream::Environment => "environment",
            Stream::Npc => "npc",
            Stream::Elections => "elections",
            Stream::Polling => "polling",
            Stream::Director => "director",
            Stream::Dialogue => "dialogue",
            Stream::Player => "player",
            Stream::Misc => "misc",
            Stream::Media => "media",
            Stream::Asks => "asks",
            Stream::Policy => "policy",
        }
    }
}

/// A deterministic generator with portable sampling helpers.
#[derive(Clone, Debug)]
pub struct SimRng {
    inner: ChaCha12Rng,
}

/// Serializable generator state (seed, stream, word position).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RngState {
    pub seed: [u8; 32],
    pub stream: u64,
    /// Word position as a decimal string (u128 is not JSON-portable).
    pub word_pos: String,
}

impl SimRng {
    pub fn from_seed_bytes(seed: [u8; 32]) -> Self {
        SimRng {
            inner: ChaCha12Rng::from_seed(seed),
        }
    }

    pub fn state(&self) -> RngState {
        RngState {
            seed: self.inner.get_seed(),
            stream: self.inner.get_stream(),
            word_pos: self.inner.get_word_pos().to_string(),
        }
    }

    pub fn from_state(s: &RngState) -> Option<Self> {
        let mut inner = ChaCha12Rng::from_seed(s.seed);
        inner.set_stream(s.stream);
        inner.set_word_pos(s.word_pos.parse::<u128>().ok()?);
        Some(SimRng { inner })
    }

    pub fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `[0, 1)` with 53 bits of precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform integer in `[0, n)`; returns 0 when `n == 0`. Unbiased (rejection).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let zone = u64::MAX - (u64::MAX % n) - 1;
        loop {
            let v = self.next_u64();
            if v <= zone {
                return v % n;
            }
        }
    }

    /// Uniform integer in `[lo, hi]` inclusive.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo) as u64 + 1;
        lo + self.below(span) as i64
    }

    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }

    /// Bernoulli trial; `p` is clamped to `[0, 1]`.
    pub fn chance(&mut self, p: f64) -> bool {
        self.next_f64() < crate::math::clamp01(p)
    }

    /// Normal sample via Box–Muller (libm-backed, portable).
    pub fn normal(&mut self, mean: f64, sd: f64) -> f64 {
        let u1 = 1.0 - self.next_f64(); // (0, 1]
        let u2 = self.next_f64();
        let r = crate::math::sqrt(-2.0 * crate::math::ln(u1));
        mean + sd * r * crate::math::cos(std::f64::consts::TAU * u2)
    }

    /// Index chosen proportionally to non-negative weights; `None` if all zero.
    pub fn weighted_index(&mut self, weights: &[f64]) -> Option<usize> {
        let total: f64 = weights.iter().filter(|w| w.is_finite() && **w > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut t = self.next_f64() * total;
        let mut last = None;
        for (i, &w) in weights.iter().enumerate() {
            if w.is_finite() && w > 0.0 {
                last = Some(i);
                if t < w {
                    return Some(i);
                }
                t -= w;
            }
        }
        last
    }

    /// Fisher–Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            items.swap(i, j);
        }
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            None
        } else {
            items.get(self.below(items.len() as u64) as usize)
        }
    }
}

/// All sequential streams for one world. Serialized in saves.
#[derive(Clone, Debug)]
pub struct RngStreams {
    world_seed: u64,
    streams: BTreeMap<Stream, SimRng>,
}

impl RngStreams {
    pub fn new(world_seed: u64) -> Self {
        let streams = Stream::ALL
            .iter()
            .map(|s| {
                (
                    *s,
                    SimRng::from_seed_bytes(derive_seed(world_seed, s.tag())),
                )
            })
            .collect();
        RngStreams {
            world_seed,
            streams,
        }
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    #[allow(clippy::expect_used)]
    pub fn get(&mut self, s: Stream) -> &mut SimRng {
        // Every stream is created in `new` and `from_states`; absence is a bug.
        self.streams.get_mut(&s).expect("all streams initialised")
    }

    /// Order-independent generator for lazily evaluated entities.
    pub fn keyed(&self, s: Stream, parts: &[u64]) -> SimRng {
        keyed(self.world_seed, s, parts)
    }

    pub fn states(&self) -> BTreeMap<Stream, RngState> {
        self.streams.iter().map(|(k, v)| (*k, v.state())).collect()
    }

    pub fn from_states(world_seed: u64, states: &BTreeMap<Stream, RngState>) -> Option<Self> {
        let mut fresh = RngStreams::new(world_seed);
        for (k, st) in states {
            fresh.streams.insert(*k, SimRng::from_state(st)?);
        }
        Some(fresh)
    }
}

/// Keyed, counter-style generator: identical output for identical inputs,
/// regardless of when or whether other draws happened.
pub fn keyed(world_seed: u64, s: Stream, parts: &[u64]) -> SimRng {
    let mut rng = SimRng::from_seed_bytes(derive_seed(world_seed, &format!("keyed:{}", s.tag())));
    rng.inner.set_stream(derive_u64(s.tag(), parts));
    rng
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn streams_are_independent() {
        let mut a = RngStreams::new(42);
        let mut b = RngStreams::new(42);
        // Extra draws on one stream must not affect another.
        for _ in 0..10 {
            a.get(Stream::Economy).next_u64();
        }
        assert_eq!(a.get(Stream::Npc).next_u64(), b.get(Stream::Npc).next_u64());
    }

    #[test]
    fn state_round_trip_continues_sequence() {
        let mut s = RngStreams::new(7);
        for _ in 0..5 {
            s.get(Stream::Elections).next_u64();
        }
        let saved = s.states();
        let mut restored = RngStreams::from_states(7, &saved).expect("restore");
        assert_eq!(
            s.get(Stream::Elections).next_u64(),
            restored.get(Stream::Elections).next_u64()
        );
    }

    #[test]
    fn keyed_is_order_independent() {
        let streams = RngStreams::new(99);
        let x1 = streams.keyed(Stream::Npc, &[5, 2031]).next_u64();
        let _ = streams.keyed(Stream::Npc, &[6, 2031]).next_u64();
        let x2 = streams.keyed(Stream::Npc, &[5, 2031]).next_u64();
        assert_eq!(x1, x2);
    }

    proptest! {
        #[test]
        fn f64_in_unit_interval(seed in any::<u64>()) {
            let mut r = RngStreams::new(seed);
            for _ in 0..64 {
                let v = r.get(Stream::Misc).next_f64();
                prop_assert!((0.0..1.0).contains(&v));
            }
        }

        #[test]
        fn below_in_range(seed in any::<u64>(), n in 1u64..1000) {
            let mut r = RngStreams::new(seed);
            for _ in 0..32 {
                prop_assert!(r.get(Stream::Misc).below(n) < n);
            }
        }
    }
}

// Serialized as `{ world_seed, states }` so saves can restore every stream.
#[derive(Serialize, Deserialize)]
struct RngStreamsRepr {
    world_seed: u64,
    states: BTreeMap<Stream, RngState>,
}

impl Serialize for RngStreams {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        RngStreamsRepr {
            world_seed: self.world_seed,
            states: self.states(),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for RngStreams {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let r = RngStreamsRepr::deserialize(d)?;
        RngStreams::from_states(r.world_seed, &r.states)
            .ok_or_else(|| serde::de::Error::custom("invalid RNG state"))
    }
}
