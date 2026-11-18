//! Stable, portable hashing for state hashes, content hashes, and RNG keys.
//! Never uses `std::hash` with random keys (DEC-041 rule 5).

use serde::Serialize;

/// 32-byte BLAKE3 digest rendered as lowercase hex.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StableHash(pub [u8; 32]);

impl StableHash {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        StableHash(*blake3::hash(bytes).as_bytes())
    }

    pub fn hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    /// First 16 hex chars, for display.
    pub fn short(&self) -> String {
        self.hex()[..16].to_string()
    }
}

/// Hash a value through its canonical JSON serialization. All simulation
/// state uses ordered collections, so serialization is deterministic.
pub fn stable_hash_json<T: Serialize>(value: &T) -> Result<StableHash, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(StableHash::of_bytes(&bytes))
}

/// Derive a `u64` key from a domain tag and integer parts (RNG stream keys).
pub fn derive_u64(tag: &str, parts: &[u64]) -> u64 {
    let mut h = blake3::Hasher::new();
    h.update(tag.as_bytes());
    for p in parts {
        h.update(&p.to_le_bytes());
    }
    let out = h.finalize();
    let b = out.as_bytes();
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Derive a 32-byte seed from a world seed and a domain tag.
pub fn derive_seed(world_seed: u64, tag: &str) -> [u8; 32] {
    let mut h = blake3::Hasher::new_derive_key("mandate rng seed v1");
    h.update(&world_seed.to_le_bytes());
    h.update(tag.as_bytes());
    *h.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derivations_are_stable() {
        assert_eq!(derive_u64("x", &[1, 2]), derive_u64("x", &[1, 2]));
        assert_ne!(derive_u64("x", &[1, 2]), derive_u64("x", &[2, 1]));
        assert_ne!(derive_seed(1, "a"), derive_seed(1, "b"));
    }
}
