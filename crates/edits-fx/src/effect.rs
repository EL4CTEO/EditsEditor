//! Effect definitions: a WGSL file with a TOML header in `//!` comment lines.
//!
//! ```wgsl
//! //! id = "rgb_split"
//! //! name = "RGB Split"
//! //! kind = "filter"            # filter | transition | generator
//! //! category = "glitch"
//! //! description = "Offsets the red and blue channels."
//! //! tags = ["glitch", "amv"]
//! //! params = [
//! //!   { name = "amount", type = "float", default = 12.0, min = 0.0, max = 200.0, desc = "Offset in pixels" },
//! //!   { name = "angle",  type = "angle", default = 0.0, desc = "Direction in degrees" },
//! //! ]
//! fn effect(uv: vec2f, p: Params) -> vec4f {
//!     let d = vec2f(cos(radians(p.angle)), sin(radians(p.angle))) * p.amount * texel();
//!     let c = src(uv);
//!     return vec4f(src(uv + d).r, c.g, src(uv - d).b, c.a);
//! }
//! ```
//!
//! The engine generates a `Params` struct from the header and calls the entry point(s).
//! Multi-pass effects list `passes = [{ entry = "...", scale = 0.5, inputs = ["prev", "input", "0"], data = [..] }]`.

use serde::{Deserialize, Serialize};

use edits_core::{Color, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    /// Processes one image.
    Filter,
    /// Blends from one image (`from_img`) to another (`to_img`) over `progress()`.
    Transition,
    /// Creates an image from nothing (used by generator clips).
    Generator,
}

impl EffectKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EffectKind::Filter => "filter",
            EffectKind::Transition => "transition",
            EffectKind::Generator => "generator",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    Float,
    Int,
    Bool,
    /// Angle in degrees (passed as degrees; use `radians()` in WGSL).
    Angle,
    Vec2,
    Vec3,
    /// Normalized position (0..1 uv, [0.5, 0.5] = center).
    Point,
    Color,
    /// One of `options` (string), passed to WGSL as the option index (f32).
    Enum,
    /// Image asset id, bound to texture `t2` (`extra(uv)`).
    Image,
    /// .cube LUT asset id, bound to texture `t2` unwrapped as a 2D strip.
    Lut,
    /// Free text (presets only; not passed to shaders).
    String,
}

impl ParamType {
    fn wgsl(&self) -> Option<(&'static str, &'static str)> {
        Some(match self {
            ParamType::Float | ParamType::Int | ParamType::Bool | ParamType::Angle | ParamType::Enum => ("f32", ".x"),
            ParamType::Vec2 | ParamType::Point => ("vec2f", ".xy"),
            ParamType::Vec3 => ("vec3f", ".xyz"),
            ParamType::Color => ("vec4f", ""),
            ParamType::Image | ParamType::Lut | ParamType::String => return None,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type", default = "float_ty")]
    pub ty: ParamType,
    #[serde(default)]
    pub default: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, alias = "description", skip_serializing_if = "String::is_empty")]
    pub desc: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

fn float_ty() -> ParamType {
    ParamType::Float
}

impl ParamDef {
    /// Convert a value to the 4-float uniform slot.
    pub fn to_slot(&self, v: &Value) -> [f32; 4] {
        let f = |x: f64| x as f32;
        match self.ty {
            ParamType::Float | ParamType::Angle => [f(v.as_f64().unwrap_or_else(|| self.default.as_f64().unwrap_or(0.0))), 0.0, 0.0, 0.0],
            ParamType::Int => [f(v.as_f64().unwrap_or(0.0).round()), 0.0, 0.0, 0.0],
            ParamType::Bool => [if v.as_bool().unwrap_or(false) { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
            ParamType::Enum => {
                let idx = match v {
                    Value::Str(s) => self.options.iter().position(|o| o.eq_ignore_ascii_case(s)).unwrap_or(0) as f64,
                    other => other.as_f64().unwrap_or(0.0).round(),
                };
                [f(idx), 0.0, 0.0, 0.0]
            }
            ParamType::Vec2 | ParamType::Point | ParamType::Vec3 => {
                let a = v.to_vec4().unwrap_or([0.0; 4]);
                let a = match v {
                    Value::Num(n) => [*n, *n, *n, 0.0],
                    _ => a,
                };
                [f(a[0]), f(a[1]), f(a[2]), 0.0]
            }
            ParamType::Color => {
                let c = v.as_color().or_else(|| self.default.as_color()).unwrap_or(Color::WHITE);
                let [r, g, b, a] = c.0;
                [edits_core::color::srgb_to_linear(r), edits_core::color::srgb_to_linear(g), edits_core::color::srgb_to_linear(b), a]
            }
            ParamType::Image | ParamType::Lut | ParamType::String => [0.0; 4],
        }
    }

    pub fn is_texture(&self) -> bool {
        matches!(self.ty, ParamType::Image | ParamType::Lut)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PassDef {
    /// WGSL function name: `fn <entry>(uv: vec2f, p: Params) -> vec4f`.
    pub entry: String,
    /// Optional name other passes can reference in `inputs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Output resolution relative to the effect output (e.g. 0.5 for half-res blur passes).
    #[serde(default = "one_f32")]
    pub scale: f32,
    /// Texture inputs t0, t1, t2: "prev", "input", "none", a pass index ("0") or pass name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    /// Constants available as `U.g.pass_data`.
    #[serde(default)]
    pub data: [f32; 4],
    /// Repeat this pass N times (ping-ponging), e.g. for iterative blurs.
    #[serde(default = "one_u32")]
    pub repeat: u32,
}

fn one_f32() -> f32 {
    1.0
}
fn one_u32() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize)]
struct Header {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default = "filter_kind")]
    kind: EffectKind,
    #[serde(default)]
    category: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    params: Vec<ParamDef>,
    #[serde(default)]
    passes: Vec<PassDef>,
    #[serde(default)]
    author: String,
}

fn filter_kind() -> EffectKind {
    EffectKind::Filter
}

#[derive(Clone, Debug, Serialize)]
pub struct EffectDef {
    pub id: String,
    pub name: String,
    pub kind: EffectKind,
    pub category: String,
    pub description: String,
    pub tags: Vec<String>,
    pub params: Vec<ParamDef>,
    pub passes: Vec<PassDef>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// The full original source (header + WGSL).
    #[serde(skip)]
    pub source: String,
    pub builtin: bool,
}

pub const MAX_PARAM_SLOTS: usize = 32;

#[derive(Debug, thiserror::Error)]
pub enum FxError {
    #[error("header: {0}")]
    Header(String),
    #[error("{0}")]
    Invalid(String),
    #[error("WGSL error:\n{0}")]
    Shader(String),
}

/// Split `//!` header lines from a source file and parse them as TOML.
pub fn split_header(source: &str) -> (String, String) {
    let mut header = String::new();
    let mut body = String::new();
    for line in source.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("//!") {
            header.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            header.push('\n');
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    (header, body)
}

pub(crate) fn toml_to_json(v: toml::Value) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

impl EffectDef {
    /// Parse an effect file. `fallback_id` is used when the header has no `id`.
    pub fn parse(source: &str, fallback_id: &str, builtin: bool) -> Result<EffectDef, FxError> {
        let (header, _) = split_header(source);
        let raw: toml::Value = toml::from_str(&header).map_err(|e| FxError::Header(e.to_string()))?;
        let h: Header = serde_json::from_value(toml_to_json(raw)).map_err(|e| FxError::Header(e.to_string()))?;
        let id = if h.id.is_empty() { fallback_id.to_string() } else { h.id };
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.') {
            return Err(FxError::Invalid(format!("invalid effect id '{id}'")));
        }
        let mut seen = std::collections::HashSet::new();
        let mut slots = 0;
        let mut textures = 0;
        for p in &h.params {
            if !valid_ident(&p.name) {
                return Err(FxError::Invalid(format!("param name '{}' must be a snake_case identifier", p.name)));
            }
            if !seen.insert(p.name.clone()) {
                return Err(FxError::Invalid(format!("duplicate param '{}'", p.name)));
            }
            if p.is_texture() {
                textures += 1;
            } else if p.ty != ParamType::String {
                slots += 1;
            }
            if p.ty == ParamType::Enum && p.options.is_empty() {
                return Err(FxError::Invalid(format!("enum param '{}' needs options", p.name)));
            }
        }
        if slots > MAX_PARAM_SLOTS {
            return Err(FxError::Invalid(format!("too many params ({slots} > {MAX_PARAM_SLOTS})")));
        }
        if textures > 1 {
            return Err(FxError::Invalid("at most one image/lut param per effect".into()));
        }
        let name = if h.name.is_empty() { id.replace('_', " ") } else { h.name };
        let passes = if h.passes.is_empty() {
            vec![PassDef { entry: "effect".into(), name: None, scale: 1.0, inputs: vec![], data: [0.0; 4], repeat: 1 }]
        } else {
            h.passes
        };
        for p in &passes {
            if !valid_ident(&p.entry) {
                return Err(FxError::Invalid(format!("invalid pass entry '{}'", p.entry)));
            }
            if !(0.01..=4.0).contains(&p.scale) {
                return Err(FxError::Invalid(format!("pass scale {} out of range 0.01..4", p.scale)));
            }
        }
        Ok(EffectDef {
            id,
            name,
            kind: h.kind,
            category: if h.category.is_empty() { "custom".into() } else { h.category },
            description: h.description,
            tags: h.tags,
            params: h.params,
            passes,
            author: h.author,
            source: source.to_string(),
            builtin,
        })
    }

    pub fn param(&self, name: &str) -> Option<&ParamDef> {
        self.params.iter().find(|p| p.name == name)
    }

    /// Params that occupy uniform slots, in slot order.
    pub fn slot_params(&self) -> impl Iterator<Item = &ParamDef> {
        self.params.iter().filter(|p| p.ty.wgsl().is_some())
    }

    pub fn texture_param(&self) -> Option<&ParamDef> {
        self.params.iter().find(|p| p.is_texture())
    }

    /// WGSL code (without the `//!` header semantics; header lines stay as comments so line
    /// numbers in errors match the file).
    pub fn code(&self) -> &str {
        &self.source
    }

    /// Unique entry points (one fragment entry per distinct pass entry).
    pub fn entries(&self) -> Vec<String> {
        let mut v: Vec<String> = vec![];
        for p in &self.passes {
            if !v.contains(&p.entry) {
                v.push(p.entry.clone());
            }
        }
        v
    }

    /// Fragment entry point name for a pass entry function.
    pub fn fragment_name(entry: &str) -> String {
        format!("fs_{entry}")
    }

    /// Full WGSL module: user code first (so error line numbers match the file), then the
    /// prelude, the generated `Params` plumbing and one fragment entry point per pass entry.
    pub fn module_source(&self) -> String {
        let mut s = String::with_capacity(self.source.len() + crate::PRELUDE.len() + 1024);
        s.push_str(&self.source);
        s.push_str("\n\n");
        s.push_str(crate::PRELUDE);
        s.push_str("\n// ---- generated ----\nstruct Params {\n");
        let mut any = false;
        for p in self.slot_params() {
            let (ty, _) = p.ty.wgsl().unwrap();
            s.push_str(&format!("    {}: {},\n", p.name, ty));
            any = true;
        }
        if !any {
            s.push_str("    _unused: f32,\n");
        }
        s.push_str("};\n\nfn load_params() -> Params {\n    return Params(");
        let mut args = vec![];
        for (i, p) in self.slot_params().enumerate() {
            let (_, sw) = p.ty.wgsl().unwrap();
            args.push(format!("U.p[{i}]{sw}"));
        }
        if args.is_empty() {
            args.push("0.0".into());
        }
        s.push_str(&args.join(", "));
        s.push_str(");\n}\n");
        for e in self.entries() {
            s.push_str(&format!(
                "\n@fragment\nfn {}(in: VsOut) -> @location(0) vec4f {{\n    return {}(in.uv, load_params());\n}}\n",
                Self::fragment_name(&e),
                e
            ));
        }
        s
    }

    /// Validate the generated module with naga. Errors reference line numbers of the effect file.
    pub fn validate(&self) -> Result<(), FxError> {
        let src = self.module_source();
        let module = naga::front::wgsl::parse_str(&src).map_err(|e| FxError::Shader(e.emit_to_string(&src)))?;
        let mut v = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::default());
        v.validate(&module).map_err(|e| FxError::Shader(e.emit_to_string(&src)))?;
        // check that each entry exists with the right signature
        for e in self.entries() {
            if !module.functions.iter().any(|(_, f)| f.name.as_deref() == Some(e.as_str())) {
                return Err(FxError::Invalid(format!("entry function '{e}' not found")));
            }
        }
        Ok(())
    }

    /// Default values as a map.
    pub fn defaults(&self) -> serde_json::Map<String, serde_json::Value> {
        self.params.iter().map(|p| (p.name.clone(), serde_json::to_value(&p.default).unwrap_or_default())).collect()
    }
}

pub(crate) fn valid_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(ch) if ch.is_ascii_lowercase() || ch == '_')
        && c.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"//! id = "test_fx"
//! kind = "filter"
//! params = [
//!   { name = "amount", type = "float", default = 2.0 },
//!   { name = "tint", type = "color", default = "#ff0000" },
//!   { name = "mode", type = "enum", options = ["a", "b"], default = "b" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    return src(uv) * p.amount * p.tint;
}
"##;

    #[test]
    fn parse_and_validate() {
        let d = EffectDef::parse(SAMPLE, "x", false).unwrap();
        assert_eq!(d.id, "test_fx");
        assert_eq!(d.params.len(), 3);
        d.validate().unwrap();
        let slot = d.params[2].to_slot(&Value::Str("b".into()));
        assert_eq!(slot[0], 1.0);
    }

    #[test]
    fn bad_shader_reports_line() {
        let bad = SAMPLE.replace("src(uv) * p.amount", "src(uv) * p.nope");
        let d = EffectDef::parse(&bad, "x", false).unwrap();
        let e = d.validate().unwrap_err().to_string();
        assert!(e.contains("nope"), "{e}");
    }
}
