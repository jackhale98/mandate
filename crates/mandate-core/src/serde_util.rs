//! Serde helpers for state shapes that JSON cannot express directly.

/// Serialize a `BTreeMap` with non-string keys (e.g. tuples) as a list of
/// `[key, value]` pairs. JSON object keys must be strings, and saves and the
/// state hash are JSON (DEC-044), so tuple-keyed maps need this.
pub mod map_as_pairs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<K, V, S>(map: &BTreeMap<K, V>, s: S) -> Result<S::Ok, S::Error>
    where
        K: Serialize,
        V: Serialize,
        S: Serializer,
    {
        s.collect_seq(map.iter())
    }

    pub fn deserialize<'de, K, V, D>(d: D) -> Result<BTreeMap<K, V>, D::Error>
    where
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Ok(Vec::<(K, V)>::deserialize(d)?.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct T {
        #[serde(with = "super::map_as_pairs")]
        m: BTreeMap<(u32, u32), f32>,
    }

    #[test]
    fn tuple_keys_round_trip_through_json() {
        let t = T {
            m: [((1, 2), 0.5), ((0, 9), -1.25)].into_iter().collect(),
        };
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"{"m":[[[0,9],-1.25],[[1,2],0.5]]}"#);
        assert_eq!(serde_json::from_str::<T>(&json).unwrap(), t);
    }
}
