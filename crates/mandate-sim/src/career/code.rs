//! The world code (M6 note 23): what someone else needs to play the same
//! opening, as one short line to paste. It carries the seed, the world's
//! size, the scenario, the constitution, the storyteller, the starting
//! knowledge and the opening (the draft's choice: how far the world was
//! advanced, the town, the arc), and the build it was made under, so that a
//! code from another build can say so.
//!
//! The form, upper case, fields joined by `.`:
//!
//! ```text
//! M1.<seed>.<size><knowledge>.<scenario>.<constitution>.<storyteller>.<opening>.<build>.<check>
//! ```
//!
//! - numbers are base 36 (`0-9A-Z`); content ids are four base-36 digits
//!   of their stable hash, resolved against this build's registries (a
//!   code names an id only if this build has it, or says it cannot);
//! - `-` for none (a random country, the default storyteller, play begun
//!   as the world was generated);
//! - the opening is `<months>_<town>_<arc>`;
//! - `build` is five digits of the hash of the engine version and the
//!   content hash; `check` three of the hash of everything before it.
//!
//! Pure: no state, no randomness. Reading a code is case-insensitive and
//! ignores spaces around it.

use crate::arcs::opening::OpeningChoice;
use crate::state::WorldSize;
use mandate_content::ContentBundle;
use mandate_core::hash::StableHash;

/// The form's version (the first field).
pub const VERSION: &str = "M1";

/// A world to play, as a code carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldCode {
    pub seed: u64,
    pub size: WorldSize,
    pub scenario: Option<String>,
    pub constitution: String,
    pub storyteller: Option<String>,
    pub local_knowledge: bool,
    pub opening: Option<OpeningChoice>,
    /// [`build_tag`] of the build the code was made under.
    pub build: String,
}

/// Why a code cannot be read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeError {
    /// Not a code of this form (wrong fields, not base 36).
    Malformed,
    /// A later form of the code than this build reads.
    Version,
    /// Mistyped: the check digits do not match.
    Check,
    /// It names a scenario, constitution, storyteller or arc this build
    /// does not have (`other_build`: it was made under another build).
    Unknown { other_build: bool },
}

impl CodeError {
    /// The reason's key (`ui-code-error-<key>`).
    pub fn key(self) -> &'static str {
        match self {
            CodeError::Malformed => "malformed",
            CodeError::Version => "version",
            CodeError::Check => "check",
            CodeError::Unknown { other_build: true } => "other_build",
            CodeError::Unknown { other_build: false } => "unknown",
        }
    }
}

const DIGITS: &[u8; 36] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn b36(mut n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while n > 0 {
        out.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn from_b36(s: &str) -> Option<u64> {
    if s.is_empty() || s.len() > 13 {
        return None;
    }
    let mut n: u64 = 0;
    for c in s.bytes() {
        let d = match c {
            b'0'..=b'9' => c - b'0',
            b'A'..=b'Z' => c - b'A' + 10,
            _ => return None,
        };
        n = n.checked_mul(36)?.checked_add(u64::from(d))?;
    }
    Some(n)
}

/// `width` base-36 digits of the stable hash of `tag` and `text`.
fn digest(tag: &str, text: &str, width: u32) -> String {
    let h = StableHash::of_bytes(format!("{tag}\u{0}{text}").as_bytes()).0;
    let n = u64::from_le_bytes([h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]]);
    let s = b36(n % 36u64.pow(width));
    format!("{s:0>w$}", w = width as usize)
}

fn id_digits(kind: &str, id: &str) -> String {
    digest(kind, id, 4)
}

/// The build a code is made under: the engine version and the content.
pub fn build_tag(engine_version: &str, content_hash: &str) -> String {
    digest("build", &format!("{engine_version}/{content_hash}"), 5)
}

/// This build's tag, for a content bundle.
pub fn this_build(content: &ContentBundle) -> String {
    build_tag(mandate_core::ENGINE_VERSION, &content.content_hash)
}

fn size_char(s: WorldSize) -> char {
    match s {
        WorldSize::Small => 'S',
        WorldSize::Medium => 'M',
        WorldSize::Large => 'L',
    }
}

impl WorldCode {
    /// The code as one line.
    pub fn encode(&self) -> String {
        let none = || "-".to_string();
        let opt = |kind: &str, v: &Option<String>| {
            v.as_deref().map_or_else(none, |id| id_digits(kind, id))
        };
        let opening = self.opening.as_ref().map_or_else(none, |o| {
            format!(
                "{}_{}_{}",
                b36(u64::from(o.months_ahead)),
                b36(u64::from(o.town)),
                id_digits("arc", &o.template)
            )
        });
        let body = [
            VERSION.to_string(),
            b36(self.seed),
            format!(
                "{}{}",
                size_char(self.size),
                if self.local_knowledge { '1' } else { '0' }
            ),
            opt("scenario", &self.scenario),
            id_digits("constitution", &self.constitution),
            opt("storyteller", &self.storyteller),
            opening,
            self.build.clone(),
        ]
        .join(".");
        let check = digest("check", &body, 3);
        format!("{body}.{check}")
    }

    /// Read a code against this build's content.
    pub fn decode(code: &str, content: &ContentBundle) -> Result<WorldCode, CodeError> {
        let code = code.trim().to_ascii_uppercase();
        let parts: Vec<&str> = code.split('.').collect();
        if parts
            .first()
            .is_some_and(|v| *v != VERSION && v.starts_with('M'))
        {
            return Err(CodeError::Version);
        }
        let [
            v,
            seed,
            flags,
            scenario,
            constitution,
            storyteller,
            opening,
            build,
            check,
        ] = parts.as_slice()
        else {
            return Err(CodeError::Malformed);
        };
        if *v != VERSION {
            return Err(CodeError::Malformed);
        }
        let body = parts[..8].join(".");
        if digest("check", &body, 3) != *check {
            return Err(CodeError::Check);
        }
        let seed = from_b36(seed).ok_or(CodeError::Malformed)?;
        let mut f = flags.chars();
        let size = match f.next() {
            Some('S') => WorldSize::Small,
            Some('M') => WorldSize::Medium,
            Some('L') => WorldSize::Large,
            _ => return Err(CodeError::Malformed),
        };
        let local_knowledge = match (f.next(), f.next()) {
            (Some('0'), None) => false,
            (Some('1'), None) => true,
            _ => return Err(CodeError::Malformed),
        };
        let other_build = *build != this_build(content);
        let unknown = CodeError::Unknown { other_build };
        let find = |kind: &str, digits: &str, ids: &mut dyn Iterator<Item = &String>| {
            let mut found = ids.filter(|id| id_digits(kind, id) == digits);
            match (found.next(), found.next()) {
                (Some(id), None) => Ok(id.clone()),
                _ => Err(unknown),
            }
        };
        let optional = |kind: &str, digits: &str, ids: &mut dyn Iterator<Item = &String>| {
            if digits == "-" {
                Ok(None)
            } else {
                find(kind, digits, ids).map(Some)
            }
        };
        let scenario = optional(
            "scenario",
            scenario,
            &mut content.scenarios.iter().map(|s| &s.id),
        )?;
        let constitution = find(
            "constitution",
            constitution,
            &mut content.constitutions.keys(),
        )?;
        let storyteller = optional(
            "storyteller",
            storyteller,
            &mut content.storytellers.iter().map(|s| &s.id),
        )?;
        let opening = if *opening == "-" {
            None
        } else {
            let [months, town, arc] = opening.split('_').collect::<Vec<_>>()[..] else {
                return Err(CodeError::Malformed);
            };
            let months_ahead = from_b36(months)
                .and_then(|m| u32::try_from(m).ok())
                .ok_or(CodeError::Malformed)?;
            let town = from_b36(town)
                .and_then(|m| u32::try_from(m).ok())
                .ok_or(CodeError::Malformed)?;
            let template = find("arc", arc, &mut content.arcs.iter().map(|a| &a.id))?;
            Some(OpeningChoice {
                months_ahead,
                town,
                template,
            })
        };
        if build.len() != 5 || from_b36(build).is_none() {
            return Err(CodeError::Malformed);
        }
        Ok(WorldCode {
            seed,
            size,
            scenario,
            constitution,
            storyteller,
            local_knowledge,
            opening,
            build: build.to_string(),
        })
    }

    /// Made under this build.
    pub fn same_build(&self, content: &ContentBundle) -> bool {
        self.build == this_build(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> ContentBundle {
        let src = mandate_content::bundle::read_dir_sources(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content"),
        )
        .unwrap();
        mandate_content::load(&src).unwrap().0
    }

    fn sample(c: &ContentBundle) -> WorldCode {
        WorldCode {
            seed: 18_446_744_073_709_551_557,
            size: WorldSize::Medium,
            scenario: c.scenarios.get(1).map(|s| s.id.clone()),
            constitution: c.constitutions.keys().next().cloned().unwrap(),
            storyteller: c.storytellers.first().map(|s| s.id.clone()),
            local_knowledge: true,
            opening: Some(OpeningChoice {
                months_ahead: 4,
                town: 12,
                template: c.arcs[0].id.clone(),
            }),
            build: this_build(c),
        }
    }

    #[test]
    fn a_code_reads_back_as_it_was_made() {
        let c = content();
        let w = sample(&c);
        let code = w.encode();
        assert!(code.starts_with("M1."), "{code}");
        assert!(code.len() < 64, "{code}");
        assert_eq!(WorldCode::decode(&code, &c), Ok(w.clone()));
        // Pasted in lower case, with spaces around it.
        assert_eq!(
            WorldCode::decode(&format!("  {}\n", code.to_lowercase()), &c),
            Ok(w.clone())
        );
        assert!(w.same_build(&c));
        // A quick start: no opening, a random country, the default teller.
        let q = WorldCode {
            scenario: None,
            storyteller: None,
            opening: None,
            local_knowledge: false,
            seed: 0,
            ..w
        };
        assert_eq!(WorldCode::decode(&q.encode(), &c), Ok(q));
    }

    #[test]
    fn a_mistyped_or_foreign_code_says_why() {
        let c = content();
        let w = sample(&c);
        let code = w.encode();
        // One digit of the seed changed: the check fails.
        let mut bad: Vec<char> = code.chars().collect();
        bad[4] = if bad[4] == '7' { '8' } else { '7' };
        let bad: String = bad.into_iter().collect();
        assert_eq!(WorldCode::decode(&bad, &c), Err(CodeError::Check));
        assert_eq!(WorldCode::decode("hello", &c), Err(CodeError::Malformed));
        assert_eq!(
            WorldCode::decode("M2.1.S0.-.-.-.-.00000.000", &c),
            Err(CodeError::Version)
        );
        // Another build: read, and marked as such.
        let other = WorldCode {
            build: "ZZZZZ".into(),
            ..w.clone()
        };
        let back = WorldCode::decode(&other.encode(), &c).unwrap();
        assert!(!back.same_build(&c));
        // Another build naming an arc this one lacks: refused, saying so.
        let foreign = WorldCode {
            build: "ZZZZZ".into(),
            opening: Some(OpeningChoice {
                months_ahead: 0,
                town: 1,
                template: "arc.not_in_this_build".into(),
            }),
            ..w
        };
        assert_eq!(
            WorldCode::decode(&foreign.encode(), &c),
            Err(CodeError::Unknown { other_build: true })
        );
    }
}
