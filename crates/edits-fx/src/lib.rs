//! # edits-fx
//!
//! The effect / transition / generator / preset library. Effects are WGSL shaders with a TOML
//! header; presets are Rhai scripts with a TOML header. Built-ins are embedded at compile time;
//! projects can add their own (`custom_effects`, `custom_presets`) in the same format.

pub mod effect;
pub mod lut;
pub mod preset;

use std::sync::Arc;

use indexmap::IndexMap;
use serde::Serialize;

pub use effect::{EffectDef, EffectKind, FxError, ParamDef, ParamType, PassDef};
pub use preset::{PresetDef, PresetScope};

/// Shared WGSL prelude prepended to every effect module.
pub const PRELUDE: &str = include_str!("prelude.wgsl");

mod builtin {
    include!(concat!(env!("OUT_DIR"), "/builtin.rs"));
}

fn stem(path: &str) -> String {
    let f = path.rsplit('/').next().unwrap_or(path);
    f.split('.').next().unwrap_or(f).to_string()
}

/// The effect & preset registry.
#[derive(Clone, Default)]
pub struct Library {
    effects: IndexMap<String, Arc<EffectDef>>,
    presets: IndexMap<String, Arc<PresetDef>>,
    /// Errors from loading custom definitions (id -> message).
    pub errors: IndexMap<String, String>,
}

static BUILTIN: std::sync::OnceLock<Library> = std::sync::OnceLock::new();

/// Compact listing entry.
#[derive(Clone, Debug, Serialize)]
pub struct EffectSummary {
    pub id: String,
    pub name: String,
    pub kind: EffectKind,
    pub category: String,
    pub description: String,
    pub params: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PresetSummary {
    pub id: String,
    pub name: String,
    pub scope: PresetScope,
    pub category: String,
    pub description: String,
    pub params: Vec<String>,
}

impl Library {
    /// The built-in library (parsed once).
    pub fn builtin() -> &'static Library {
        BUILTIN.get_or_init(|| {
            let mut lib = Library::default();
            for (path, src) in builtin::BUILTIN_EFFECTS {
                match EffectDef::parse(src, &stem(path), true) {
                    Ok(d) => {
                        lib.effects.insert(d.id.clone(), Arc::new(d));
                    }
                    Err(e) => {
                        lib.errors.insert(path.to_string(), e.to_string());
                    }
                }
            }
            for (path, src) in builtin::BUILTIN_PRESETS {
                match PresetDef::parse(src, &stem(path), true) {
                    Ok(d) => {
                        lib.presets.insert(d.id.clone(), Arc::new(d));
                    }
                    Err(e) => {
                        lib.errors.insert(path.to_string(), e.to_string());
                    }
                }
            }
            lib
        })
    }

    /// Built-ins plus a project's custom effects and presets (customs override built-ins).
    pub fn for_project(project: &edits_core::Project) -> Library {
        let mut lib = Library::builtin().clone();
        lib.errors.clear();
        for (id, src) in &project.custom_effects {
            match EffectDef::parse(src, id, false) {
                Ok(mut d) => {
                    d.id = id.clone();
                    lib.effects.insert(id.clone(), Arc::new(d));
                }
                Err(e) => {
                    lib.errors.insert(id.clone(), e.to_string());
                }
            }
        }
        for (id, src) in &project.custom_presets {
            match PresetDef::parse(src, id, false) {
                Ok(mut d) => {
                    d.id = id.clone();
                    lib.presets.insert(id.clone(), Arc::new(d));
                }
                Err(e) => {
                    lib.errors.insert(id.clone(), e.to_string());
                }
            }
        }
        lib
    }

    pub fn effect(&self, id: &str) -> Option<&Arc<EffectDef>> {
        self.effects.get(id)
    }

    pub fn preset(&self, id: &str) -> Option<&Arc<PresetDef>> {
        self.presets.get(id)
    }

    pub fn effects(&self) -> impl Iterator<Item = &Arc<EffectDef>> {
        self.effects.values()
    }

    pub fn presets(&self) -> impl Iterator<Item = &Arc<PresetDef>> {
        self.presets.values()
    }

    pub fn effect_count(&self) -> usize {
        self.effects.len()
    }

    pub fn preset_count(&self) -> usize {
        self.presets.len()
    }

    /// Search effects by kind / category / free text (matches id, name, tags, description).
    pub fn search_effects(&self, kind: Option<EffectKind>, category: Option<&str>, query: Option<&str>) -> Vec<EffectSummary> {
        let q = query.map(|q| q.to_ascii_lowercase());
        self.effects
            .values()
            .filter(|e| kind.is_none_or(|k| e.kind == k))
            .filter(|e| category.is_none_or(|c| e.category.eq_ignore_ascii_case(c)))
            .filter(|e| {
                q.as_ref().is_none_or(|q| {
                    q.split_whitespace().all(|w| {
                        e.id.contains(w)
                            || e.name.to_ascii_lowercase().contains(w)
                            || e.description.to_ascii_lowercase().contains(w)
                            || e.tags.iter().any(|t| t.contains(w))
                            || e.category.contains(w)
                    })
                })
            })
            .map(|e| EffectSummary {
                id: e.id.clone(),
                name: e.name.clone(),
                kind: e.kind,
                category: e.category.clone(),
                description: e.description.clone(),
                params: e.params.iter().map(|p| p.name.clone()).collect(),
            })
            .collect()
    }

    pub fn search_presets(&self, category: Option<&str>, query: Option<&str>) -> Vec<PresetSummary> {
        let q = query.map(|q| q.to_ascii_lowercase());
        self.presets
            .values()
            .filter(|e| category.is_none_or(|c| e.category.eq_ignore_ascii_case(c)))
            .filter(|e| {
                q.as_ref().is_none_or(|q| {
                    q.split_whitespace().all(|w| {
                        e.id.contains(w)
                            || e.name.to_ascii_lowercase().contains(w)
                            || e.description.to_ascii_lowercase().contains(w)
                            || e.tags.iter().any(|t| t.contains(w))
                            || e.category.contains(w)
                    })
                })
            })
            .map(|e| PresetSummary {
                id: e.id.clone(),
                name: e.name.clone(),
                scope: e.scope,
                category: e.category.clone(),
                description: e.description.clone(),
                params: e.params.iter().map(|p| p.name.clone()).collect(),
            })
            .collect()
    }

    /// Categories with counts: (kind, category, count).
    pub fn effect_categories(&self) -> Vec<(EffectKind, String, usize)> {
        let mut m: IndexMap<(EffectKind, String), usize> = IndexMap::new();
        for e in self.effects.values() {
            *m.entry((e.kind, e.category.clone())).or_default() += 1;
        }
        let mut v: Vec<_> = m.into_iter().map(|((k, c), n)| (k, c, n)).collect();
        v.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()).then(a.1.cmp(&b.1)));
        v
    }

    pub fn preset_categories(&self) -> Vec<(String, usize)> {
        let mut m: IndexMap<String, usize> = IndexMap::new();
        for p in self.presets.values() {
            *m.entry(p.category.clone()).or_default() += 1;
        }
        let mut v: Vec<_> = m.into_iter().collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_parse_and_validate() {
        let lib = Library::builtin();
        assert!(lib.errors.is_empty(), "load errors: {:#?}", lib.errors);
        let mut failures = vec![];
        for e in lib.effects() {
            if let Err(err) = e.validate() {
                failures.push(format!("== {} ==\n{err}", e.id));
            }
        }
        assert!(failures.is_empty(), "{} invalid effects:\n{}", failures.len(), failures.join("\n"));
        println!("{} effects, {} presets", lib.effect_count(), lib.preset_count());
    }
}
