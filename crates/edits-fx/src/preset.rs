//! Presets: Rhai scripts with a `//!` TOML header. A preset edits a clip (scope "clip") or the
//! whole project (scope "timeline"). Built-in presets ship as `.rhai` files and agents can write
//! their own in exactly the same format.
//!
//! ```rhai
//! //! id = "zoom_punch"
//! //! name = "Zoom Punch"
//! //! scope = "clip"
//! //! category = "motion"
//! //! description = "Quick scale punch at a hit point."
//! //! params = [
//! //!   { name = "at", type = "float", default = 0.0, desc = "clip-local time of the hit (s)" },
//! //!   { name = "strength", type = "float", default = 0.2 },
//! //! ]
//! let s = 1.0 + args.strength;
//! clip.transform.scale = keys([[args.at, s], [args.at + 0.25, 1.0, "punch"]]);
//! ```
//!
//! Script environment (provided by the engine): `clip` (mutable map, clip scope), `project`
//! (mutable map, timeline scope), `args` (params with defaults applied), `ctx` (fps, width,
//! height, duration, beats near the clip...), plus helper functions (`fx`, `keys`, `add_fx`,
//! `beats_between`, `new_id`, ...).

use serde::{Deserialize, Serialize};

use crate::effect::{FxError, ParamDef, split_header, toml_to_json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetScope {
    /// Operates on one clip (`clip` variable).
    Clip,
    /// Operates on the whole project (`project` variable), e.g. auto-edits and beat montages.
    Timeline,
}

#[derive(Clone, Debug, Serialize)]
pub struct PresetDef {
    pub id: String,
    pub name: String,
    pub scope: PresetScope,
    pub category: String,
    pub description: String,
    pub tags: Vec<String>,
    pub params: Vec<ParamDef>,
    #[serde(skip)]
    pub source: String,
    pub builtin: bool,
}

#[derive(Deserialize)]
struct Header {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default = "clip_scope")]
    scope: PresetScope,
    #[serde(default)]
    category: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    params: Vec<ParamDef>,
}

fn clip_scope() -> PresetScope {
    PresetScope::Clip
}

impl PresetDef {
    pub fn parse(source: &str, fallback_id: &str, builtin: bool) -> Result<PresetDef, FxError> {
        let (header, _) = split_header(source);
        let raw: toml::Value = toml::from_str(&header).map_err(|e| FxError::Header(e.to_string()))?;
        let h: Header = serde_json::from_value(toml_to_json(raw)).map_err(|e| FxError::Header(e.to_string()))?;
        let id = if h.id.is_empty() { fallback_id.to_string() } else { h.id };
        if id.is_empty() {
            return Err(FxError::Invalid("preset needs an id".into()));
        }
        for p in &h.params {
            if !crate::effect::valid_ident(&p.name) {
                return Err(FxError::Invalid(format!("param name '{}' must be snake_case", p.name)));
            }
        }
        Ok(PresetDef {
            name: if h.name.is_empty() { id.replace('_', " ") } else { h.name },
            id,
            scope: h.scope,
            category: if h.category.is_empty() { "custom".into() } else { h.category },
            description: h.description,
            tags: h.tags,
            params: h.params,
            source: source.to_string(),
            builtin,
        })
    }

    /// Script body (header lines are comments in Rhai too, so the full source is executable).
    pub fn script(&self) -> &str {
        &self.source
    }

    /// Merge user args over defaults.
    pub fn resolve_args(
        &self,
        args: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<serde_json::Map<String, serde_json::Value>, FxError> {
        let mut out = serde_json::Map::new();
        for p in &self.params {
            let v = args.get(&p.name).cloned().unwrap_or_else(|| serde_json::to_value(&p.default).unwrap_or_default());
            out.insert(p.name.clone(), v);
        }
        for (k, v) in args {
            if !out.contains_key(k) {
                return Err(FxError::Invalid(format!(
                    "unknown preset arg '{k}' (params: {})",
                    self.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
                )));
            }
            out.insert(k.clone(), v.clone());
        }
        Ok(out)
    }
}
