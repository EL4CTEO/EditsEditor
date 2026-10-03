//! Dynamically-typed animatable values used for effect parameters and expressions.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{color::Color, math::Vec2};

/// A loosely-typed parameter value.
///
/// - numbers: `0.5`
/// - vectors / colors as arrays: `[0.2, 0.4]`, `[1, 0, 0, 1]`
/// - booleans: `true`
/// - strings: colors (`"#ff00aa"`), enum options (`"radial"`), asset ids, text
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Value {
    Bool(bool),
    Num(f64),
    Vec(Vec<f64>),
    Str(String),
}

impl Default for Value {
    fn default() -> Self {
        Value::Num(0.0)
    }
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            Value::Vec(v) => v.first().copied(),
            Value::Str(s) => s.trim().parse().ok(),
        }
    }

    /// Flatten into up to 4 floats. Strings are parsed as colors when possible.
    pub fn to_vec4(&self) -> Option<[f64; 4]> {
        match self {
            Value::Num(n) => Some([*n, *n, *n, *n]),
            Value::Bool(b) => {
                let n = if *b { 1.0 } else { 0.0 };
                Some([n, 0.0, 0.0, 0.0])
            }
            Value::Vec(v) => {
                let mut out = [0.0; 4];
                for (i, x) in v.iter().take(4).enumerate() {
                    out[i] = *x;
                }
                if v.len() == 3 {
                    out[3] = 1.0;
                }
                Some(out)
            }
            Value::Str(s) => Color::parse(s).map(|c| c.0.map(|x| x as f64)),
        }
    }

    pub fn as_color(&self) -> Option<Color> {
        match self {
            Value::Str(s) => Color::parse(s),
            Value::Vec(v) => Color::from_slice(v),
            Value::Num(n) => Some(Color([*n as f32, *n as f32, *n as f32, 1.0])),
            Value::Bool(_) => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            Value::Num(n) => Some(*n != 0.0),
            Value::Str(s) => match s.as_str() {
                "true" | "on" | "yes" => Some(true),
                "false" | "off" | "no" => Some(false),
                _ => None,
            },
            Value::Vec(_) => None,
        }
    }

    pub fn lerp(&self, o: &Value, t: f64) -> Value {
        match (self, o) {
            (Value::Num(a), Value::Num(b)) => Value::Num(a + (b - a) * t),
            (Value::Vec(a), Value::Vec(b)) => {
                let n = a.len().max(b.len());
                Value::Vec(
                    (0..n)
                        .map(|i| {
                            let x = a.get(i).or(a.last()).copied().unwrap_or(0.0);
                            let y = b.get(i).or(b.last()).copied().unwrap_or(0.0);
                            x + (y - x) * t
                        })
                        .collect(),
                )
            }
            (Value::Num(a), Value::Vec(b)) => Value::Vec(b.iter().map(|y| a + (y - a) * t).collect()),
            (Value::Vec(a), Value::Num(b)) => Value::Vec(a.iter().map(|x| x + (b - x) * t).collect()),
            (Value::Str(_), Value::Str(_)) | (Value::Vec(_), Value::Str(_)) | (Value::Str(_), Value::Vec(_)) => {
                // colors interpolate; other strings hold
                match (self.as_color(), o.as_color()) {
                    (Some(a), Some(b)) => {
                        let c = a.lerp(&b, t);
                        Value::Vec(c.0.iter().map(|x| *x as f64).collect())
                    }
                    _ => {
                        if t < 1.0 {
                            self.clone()
                        } else {
                            o.clone()
                        }
                    }
                }
            }
            _ => {
                if t < 1.0 {
                    self.clone()
                } else {
                    o.clone()
                }
            }
        }
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::Num(v)
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Str(v.to_string())
    }
}
impl From<Vec<f64>> for Value {
    fn from(v: Vec<f64>) -> Self {
        Value::Vec(v)
    }
}

/// Types that can be keyframed.
pub trait Animatable: Clone {
    fn lerp(&self, other: &Self, t: f64) -> Self;
    fn to_value(&self) -> Value;
    fn from_value(v: &Value) -> Option<Self>;
}

impl Animatable for f64 {
    fn lerp(&self, o: &Self, t: f64) -> Self {
        self + (o - self) * t
    }
    fn to_value(&self) -> Value {
        Value::Num(*self)
    }
    fn from_value(v: &Value) -> Option<Self> {
        v.as_f64()
    }
}

impl Animatable for Vec2 {
    fn lerp(&self, o: &Self, t: f64) -> Self {
        Vec2([self.0[0] + (o.0[0] - self.0[0]) * t, self.0[1] + (o.0[1] - self.0[1]) * t])
    }
    fn to_value(&self) -> Value {
        Value::Vec(self.0.to_vec())
    }
    fn from_value(v: &Value) -> Option<Self> {
        match v {
            Value::Num(n) => Some(Vec2([*n, *n])),
            Value::Vec(a) if a.len() == 1 => Some(Vec2([a[0], a[0]])),
            Value::Vec(a) if a.len() >= 2 => Some(Vec2([a[0], a[1]])),
            _ => None,
        }
    }
}

impl Animatable for Color {
    fn lerp(&self, o: &Self, t: f64) -> Self {
        Color::lerp(self, o, t)
    }
    fn to_value(&self) -> Value {
        Value::Vec(self.0.iter().map(|x| *x as f64).collect())
    }
    fn from_value(v: &Value) -> Option<Self> {
        v.as_color()
    }
}

impl Animatable for Value {
    fn lerp(&self, o: &Self, t: f64) -> Self {
        Value::lerp(self, o, t)
    }
    fn to_value(&self) -> Value {
        self.clone()
    }
    fn from_value(v: &Value) -> Option<Self> {
        Some(v.clone())
    }
}

impl Animatable for bool {
    fn lerp(&self, o: &Self, t: f64) -> Self {
        if t < 1.0 { *self } else { *o }
    }
    fn to_value(&self) -> Value {
        Value::Bool(*self)
    }
    fn from_value(v: &Value) -> Option<Self> {
        v.as_bool()
    }
}
