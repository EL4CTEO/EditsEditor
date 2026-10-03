//! Animatable properties: static values, keyframes and expressions.
//!
//! JSON forms (all equivalent styles are accepted):
//! ```json
//! "opacity": 0.5
//! "opacity": { "keyframes": [ {"t": 0, "v": 0}, {"t": 0.5, "v": 1, "ease": "ease_out_expo"} ] }
//! "opacity": { "keyframes": [ [0, 0], [0.5, 1, "ease_out_expo"] ] }
//! "opacity": { "expr": "0.5 + 0.5 * pulse(6.0)" }
//! "scale":   { "keyframes": [[0, 1.3], [0.25, 1, "punch"]], "expr": "value * (1 + 0.05 * bass())" }
//! ```
//! Keyframe times are seconds relative to the owning clip's start (clip-local time).
//! The easing on a keyframe shapes the segment from that keyframe to the next one.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

use crate::{easing::Easing, value::{Animatable, Value}};

/// Evaluates expressions attached to properties. Implemented by the engine (Rhai).
pub trait Evaluator {
    /// Evaluate `expr` at clip-local time `t`. `value` is the keyframed (pre-expression) value.
    fn eval_expr(&self, expr: &str, t: f64, value: &Value) -> Option<Value>;
}

/// An evaluator that ignores expressions.
pub struct NoExpr;
impl Evaluator for NoExpr {
    fn eval_expr(&self, _: &str, _: f64, _: &Value) -> Option<Value> {
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Property<T> {
    Static(T),
    Animated(Box<Animated<T>>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, bound(deserialize = "T: DeserializeOwned", serialize = "T: Serialize"))]
pub struct Animated<T> {
    /// Keyframes (clip-local seconds). Sorted automatically.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keyframes: Vec<Keyframe<T>>,
    /// Base value when there are no keyframes (used with `expr`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<T>,
    /// Rhai expression evaluated every frame. Has access to `value` (keyframed value), `t`,
    /// `time`, beat/audio helpers and more (see the expression reference).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    /// Repeat the keyframed section after the last keyframe.
    #[serde(default, skip_serializing_if = "LoopMode::is_none")]
    pub r#loop: LoopMode,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LoopMode {
    #[default]
    None,
    /// Repeat keyframes from the first to the last.
    Cycle,
    /// Play forward then backward.
    PingPong,
}

impl LoopMode {
    fn is_none(&self) -> bool {
        matches!(self, LoopMode::None)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Keyframe<T> {
    pub t: f64,
    pub v: T,
    pub ease: Easing,
}

impl<T> Keyframe<T> {
    pub fn new(t: f64, v: T) -> Self {
        Self { t, v, ease: Easing::Linear }
    }
    pub fn eased(t: f64, v: T, ease: Easing) -> Self {
        Self { t, v, ease }
    }
}

impl<T: Default> Default for Property<T> {
    fn default() -> Self {
        Property::Static(T::default())
    }
}

impl<T> From<T> for Property<T> {
    fn from(v: T) -> Self {
        Property::Static(v)
    }
}

impl<T: Animatable> Property<T> {
    pub fn keyframed(mut keys: Vec<Keyframe<T>>) -> Self {
        keys.sort_by(|a, b| a.t.total_cmp(&b.t));
        Property::Animated(Box::new(Animated { keyframes: keys, value: None, expr: None, r#loop: LoopMode::None }))
    }

    pub fn expression(expr: impl Into<String>, base: Option<T>) -> Self {
        Property::Animated(Box::new(Animated {
            keyframes: vec![],
            value: base,
            expr: Some(expr.into()),
            r#loop: LoopMode::None,
        }))
    }

    pub fn is_animated(&self) -> bool {
        matches!(self, Property::Animated(_))
    }

    pub fn has_expr(&self) -> bool {
        matches!(self, Property::Animated(a) if a.expr.is_some())
    }

    /// Evaluate without expressions.
    pub fn sample(&self, t: f64, default: &T) -> T {
        match self {
            Property::Static(v) => v.clone(),
            Property::Animated(a) => a.sample(t).unwrap_or_else(|| a.value.clone().unwrap_or_else(|| default.clone())),
        }
    }

    /// Full evaluation: keyframes, then expression (if any).
    pub fn eval(&self, t: f64, ev: &dyn Evaluator, default: &T) -> T {
        match self {
            Property::Static(v) => v.clone(),
            Property::Animated(a) => {
                let base = a.sample(t).unwrap_or_else(|| a.value.clone().unwrap_or_else(|| default.clone()));
                match &a.expr {
                    Some(expr) => ev
                        .eval_expr(expr, t, &base.to_value())
                        .and_then(|v| T::from_value(&v))
                        .unwrap_or(base),
                    None => base,
                }
            }
        }
    }

    /// Insert or replace a keyframe at `t`, converting a static value into keyframes if needed.
    pub fn set_keyframe(&mut self, t: f64, v: T, ease: Easing) {
        let anim = self.make_animated();
        if let Some(k) = anim.keyframes.iter_mut().find(|k| (k.t - t).abs() < 1e-9) {
            k.v = v;
            k.ease = ease;
        } else {
            anim.keyframes.push(Keyframe { t, v, ease });
            anim.keyframes.sort_by(|a, b| a.t.total_cmp(&b.t));
        }
    }

    pub fn set_expr(&mut self, expr: Option<String>) {
        if expr.is_none() && !self.is_animated() {
            return;
        }
        self.make_animated().expr = expr;
    }

    fn make_animated(&mut self) -> &mut Animated<T> {
        if let Property::Static(v) = self {
            let base = v.clone();
            *self = Property::Animated(Box::new(Animated {
                keyframes: vec![],
                value: Some(base),
                expr: None,
                r#loop: LoopMode::None,
            }));
        }
        match self {
            Property::Animated(a) => a,
            Property::Static(_) => unreachable!(),
        }
    }

    /// Times of all keyframes.
    pub fn key_times(&self) -> Vec<f64> {
        match self {
            Property::Static(_) => vec![],
            Property::Animated(a) => a.keyframes.iter().map(|k| k.t).collect(),
        }
    }

    /// Multiply all keyframe times (used when retiming clips).
    pub fn scale_time(&mut self, factor: f64) {
        if let Property::Animated(a) = self {
            for k in &mut a.keyframes {
                k.t *= factor;
            }
        }
    }

    /// Shift all keyframe times.
    pub fn shift_time(&mut self, dt: f64) {
        if let Property::Animated(a) = self {
            for k in &mut a.keyframes {
                k.t += dt;
            }
        }
    }
}

impl<T: Animatable> Animated<T> {
    /// Sample keyframes. Returns `None` if there are no keyframes.
    pub fn sample(&self, t: f64) -> Option<T> {
        let keys = &self.keyframes;
        let first = keys.first()?;
        if keys.len() == 1 {
            return Some(first.v.clone());
        }
        let last = keys.last().unwrap();
        let span = last.t - first.t;
        let mut t = t;
        if span > 1e-9 && t > last.t {
            t = match self.r#loop {
                LoopMode::None => t,
                LoopMode::Cycle => first.t + (t - first.t).rem_euclid(span),
                LoopMode::PingPong => {
                    let p = (t - first.t).rem_euclid(2.0 * span);
                    first.t + if p > span { 2.0 * span - p } else { p }
                }
            };
        }
        if t <= first.t {
            return Some(first.v.clone());
        }
        if t >= last.t {
            return Some(last.v.clone());
        }
        // binary search for the segment
        let idx = keys.partition_point(|k| k.t <= t);
        let a = &keys[idx - 1];
        let b = &keys[idx];
        let seg = (b.t - a.t).max(1e-12);
        let p = (t - a.t) / seg;
        let e = a.ease.apply(p, seg);
        Some(a.v.lerp(&b.v, e))
    }
}

// ---------- serde ----------

impl<T: Serialize> Serialize for Property<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Property::Static(v) => v.serialize(s),
            Property::Animated(a) => a.serialize(s),
        }
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Property<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        if v.is_object() {
            let mut anim: Animated<T> = serde_json::from_value(v).map_err(serde::de::Error::custom)?;
            anim.keyframes.sort_by(|a, b| a.t.total_cmp(&b.t));
            Ok(Property::Animated(Box::new(anim)))
        } else {
            serde_json::from_value(v).map(Property::Static).map_err(serde::de::Error::custom)
        }
    }
}

impl<T: JsonSchema> JsonSchema for Property<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("Property_{}", T::schema_name()).into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let st = g.subschema_for::<T>();
        let an = g.subschema_for::<Animated<T>>();
        json_schema!({
            "description": "Animatable property: a static value, or {keyframes, expr, value, loop}.",
            "anyOf": [st, an]
        })
    }
}

impl<T: Serialize> Serialize for Keyframe<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let n = if self.ease == Easing::Linear { 2 } else { 3 };
        let mut m = s.serialize_map(Some(n))?;
        m.serialize_entry("t", &self.t)?;
        m.serialize_entry("v", &self.v)?;
        if self.ease != Easing::Linear {
            m.serialize_entry("ease", &self.ease)?;
        }
        m.end()
    }
}

impl<'de, T: DeserializeOwned> Deserialize<'de> for Keyframe<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let v = serde_json::Value::deserialize(d)?;
        match v {
            serde_json::Value::Array(a) => {
                if a.len() < 2 {
                    return Err(D::Error::custom("keyframe array must be [t, v] or [t, v, ease]"));
                }
                let t = a[0].as_f64().ok_or_else(|| D::Error::custom("keyframe time must be a number"))?;
                let v = serde_json::from_value(a[1].clone()).map_err(D::Error::custom)?;
                let ease = match a.get(2) {
                    Some(e) => serde_json::from_value(e.clone()).map_err(D::Error::custom)?,
                    None => Easing::Linear,
                };
                Ok(Keyframe { t, v, ease })
            }
            serde_json::Value::Object(mut m) => {
                let t = m
                    .remove("t")
                    .or_else(|| m.remove("time"))
                    .and_then(|x| x.as_f64())
                    .ok_or_else(|| D::Error::custom("keyframe needs numeric 't'"))?;
                let raw_v = m
                    .remove("v")
                    .or_else(|| m.remove("value"))
                    .ok_or_else(|| D::Error::custom("keyframe needs 'v'"))?;
                let v = serde_json::from_value(raw_v).map_err(D::Error::custom)?;
                let ease = match m.remove("ease").or_else(|| m.remove("easing")) {
                    Some(e) => serde_json::from_value(e).map_err(D::Error::custom)?,
                    None => Easing::Linear,
                };
                if let Some(k) = m.keys().next() {
                    return Err(D::Error::custom(format!("unknown keyframe field '{k}' (expected t, v, ease)")));
                }
                Ok(Keyframe { t, v, ease })
            }
            _ => Err(D::Error::custom("keyframe must be an object {t, v, ease} or array [t, v, ease]")),
        }
    }
}

impl<T: JsonSchema> JsonSchema for Keyframe<T> {
    fn schema_name() -> Cow<'static, str> {
        format!("Keyframe_{}", T::schema_name()).into()
    }
    fn json_schema(g: &mut SchemaGenerator) -> Schema {
        let v = g.subschema_for::<T>();
        let e = g.subschema_for::<Easing>();
        json_schema!({
            "description": "Keyframe: {t, v, ease} or [t, v, ease]. t = clip-local seconds; ease shapes the segment to the next key.",
            "anyOf": [
                { "type": "object", "properties": { "t": {"type": "number"}, "v": v, "ease": e }, "required": ["t", "v"] },
                { "type": "array", "minItems": 2, "maxItems": 3 }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec2;

    #[test]
    fn static_and_keyframes() {
        let p: Property<f64> = serde_json::from_str("0.5").unwrap();
        assert_eq!(p.sample(3.0, &0.0), 0.5);

        let p: Property<f64> = serde_json::from_str(r#"{"keyframes":[[0,0],[1,10,"linear"]]}"#).unwrap();
        assert!((p.sample(0.5, &0.0) - 5.0).abs() < 1e-9);
        assert_eq!(p.sample(-1.0, &0.0), 0.0);
        assert_eq!(p.sample(2.0, &0.0), 10.0);
    }

    #[test]
    fn loops() {
        let p: Property<f64> =
            serde_json::from_str(r#"{"keyframes":[[0,0],[1,10]], "loop":"cycle"}"#).unwrap();
        assert!((p.sample(1.5, &0.0) - 5.0).abs() < 1e-9);
        let p: Property<f64> =
            serde_json::from_str(r#"{"keyframes":[[0,0],[1,10]], "loop":"ping_pong"}"#).unwrap();
        assert!((p.sample(1.25, &0.0) - 7.5).abs() < 1e-9);
    }

    #[test]
    fn vec2_keys_and_roundtrip() {
        let p: Property<Vec2> =
            serde_json::from_str(r#"{"keyframes":[{"t":0,"v":1},{"t":2,"v":[3,5],"ease":"ease_in"}]}"#).unwrap();
        let s = serde_json::to_string(&p).unwrap();
        let back: Property<Vec2> = serde_json::from_str(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn unknown_field_errors() {
        let r: Result<Property<f64>, _> = serde_json::from_str(r#"{"keyframe":[[0,1]]}"#);
        assert!(r.is_err());
    }
}
