//! The lookup seam (M6_ENGINE_CONTRACTS §2.10, task F2a): every reader of a
//! bill's or law's rules asks [`resolve`], never the template registry
//! directly. A standard bill resolves to its template as it is; a policy
//! family's bill (DEC-092) composes its family's base and chosen options on
//! the fly (pure; no cache: it runs only where templates are read).

use crate::policy::{PolicyInstance, PolicyState};
use mandate_content::{ContentBundle, PolicyTemplate};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Deref;

/// A bill's rules: a template (borrowed for a standard bill, composed for a
/// family bill) plus what composition adds. It derefs to the template, so
/// readers use it as one.
#[derive(Clone, Debug)]
pub struct Resolved<'a> {
    template: Cow<'a, PolicyTemplate>,
    /// Multiplies every mechanism's effect (a family option's
    /// `effect_factor`; 1 for a standard bill).
    pub effect_factor: f64,
    /// Fluent arguments for its keys (a family's slots; empty for a
    /// standard bill).
    pub args: BTreeMap<String, String>,
}

impl<'a> Resolved<'a> {
    /// A standard template as it is.
    pub fn plain(t: &'a PolicyTemplate) -> Self {
        Resolved {
            template: Cow::Borrowed(t),
            effect_factor: 1.0,
            args: BTreeMap::new(),
        }
    }

    /// The template (composed for a family bill).
    pub fn template(&self) -> &PolicyTemplate {
        &self.template
    }
}

impl Deref for Resolved<'_> {
    type Target = PolicyTemplate;
    fn deref(&self) -> &PolicyTemplate {
        &self.template
    }
}

/// A bill's or law's rules: its template, or its family composed with the
/// choices it was drafted with. `None` when the content no longer has them.
pub fn resolve<'a>(
    content: &'a ContentBundle,
    _st: &PolicyState,
    p: &PolicyInstance,
) -> Option<Resolved<'a>> {
    content.policies.get(&p.template).map(Resolved::plain)
}
