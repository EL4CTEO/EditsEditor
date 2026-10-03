//! Small vector types that are friendly to hand-written / agent-written JSON.
//!
//! A [`Vec2`] accepts either a single number (applied to both axes) or a
//! `[x, y]` array, so `"scale": 1.2` and `"scale": [1.2, 0.8]` both work.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 2D vector. Deserializes from a number (uniform) or a `[x, y]` array.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2(pub [f64; 2]);

impl Vec2 {
    pub const ZERO: Vec2 = Vec2([0.0, 0.0]);
    pub const ONE: Vec2 = Vec2([1.0, 1.0]);

    pub const fn new(x: f64, y: f64) -> Self {
        Self([x, y])
    }
    pub const fn splat(v: f64) -> Self {
        Self([v, v])
    }
    pub fn x(&self) -> f64 {
        self.0[0]
    }
    pub fn y(&self) -> f64 {
        self.0[1]
    }
}

impl From<[f64; 2]> for Vec2 {
    fn from(v: [f64; 2]) -> Self {
        Self(v)
    }
}

impl Serialize for Vec2 {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Vec2 {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            N(f64),
            A(Vec<f64>),
        }
        match Repr::deserialize(d)? {
            Repr::N(n) => Ok(Vec2([n, n])),
            Repr::A(a) => match a.as_slice() {
                [n] => Ok(Vec2([*n, *n])),
                [x, y, ..] => Ok(Vec2([*x, *y])),
                [] => Err(serde::de::Error::custom("empty array for vec2")),
            },
        }
    }
}

impl JsonSchema for Vec2 {
    fn schema_name() -> Cow<'static, str> {
        "Vec2".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "2D vector: a number (uniform) or [x, y].",
            "oneOf": [
                { "type": "number" },
                { "type": "array", "items": { "type": "number" }, "minItems": 1, "maxItems": 2 }
            ]
        })
    }
}

/// Linear interpolation helper.
#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// A 4x4 column-major matrix used for 2.5D layer transforms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4(pub [[f64; 4]; 4]);

impl Mat4 {
    pub const IDENTITY: Mat4 = Mat4([[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);

    pub fn mul(&self, o: &Mat4) -> Mat4 {
        let mut r = [[0.0; 4]; 4];
        for (c, col) in r.iter_mut().enumerate() {
            for (row, cell) in col.iter_mut().enumerate() {
                *cell = (0..4).map(|k| self.0[k][row] * o.0[c][k]).sum();
            }
        }
        Mat4(r)
    }

    pub fn translate(x: f64, y: f64, z: f64) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[3] = [x, y, z, 1.0];
        m
    }

    pub fn scale(x: f64, y: f64, z: f64) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[0][0] = x;
        m.0[1][1] = y;
        m.0[2][2] = z;
        m
    }

    pub fn rotate_z(rad: f64) -> Mat4 {
        let (s, c) = rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[0] = [c, s, 0.0, 0.0];
        m.0[1] = [-s, c, 0.0, 0.0];
        m
    }

    pub fn rotate_x(rad: f64) -> Mat4 {
        let (s, c) = rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[1] = [0.0, c, s, 0.0];
        m.0[2] = [0.0, -s, c, 0.0];
        m
    }

    pub fn rotate_y(rad: f64) -> Mat4 {
        let (s, c) = rad.sin_cos();
        let mut m = Self::IDENTITY;
        m.0[0] = [c, 0.0, -s, 0.0];
        m.0[2] = [s, 0.0, c, 0.0];
        m
    }

    pub fn skew_x(rad: f64) -> Mat4 {
        let mut m = Self::IDENTITY;
        m.0[1][0] = rad.tan();
        m
    }

    /// Transform a point (w = 1), returning homogeneous coordinates.
    pub fn transform(&self, p: [f64; 3]) -> [f64; 4] {
        let mut out = [0.0; 4];
        for (row, o) in out.iter_mut().enumerate() {
            *o = self.0[0][row] * p[0] + self.0[1][row] * p[1] + self.0[2][row] * p[2] + self.0[3][row];
        }
        out
    }

    pub fn to_f32(&self) -> [[f32; 4]; 4] {
        let mut r = [[0.0f32; 4]; 4];
        for c in 0..4 {
            for row in 0..4 {
                r[c][row] = self.0[c][row] as f32;
            }
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec2_parses_number_and_array() {
        let a: Vec2 = serde_json::from_str("2.5").unwrap();
        assert_eq!(a, Vec2([2.5, 2.5]));
        let b: Vec2 = serde_json::from_str("[1, 2]").unwrap();
        assert_eq!(b, Vec2([1.0, 2.0]));
    }

    #[test]
    fn mat_translate_then_point() {
        let m = Mat4::translate(10.0, 5.0, 0.0).mul(&Mat4::scale(2.0, 2.0, 1.0));
        let p = m.transform([1.0, 1.0, 0.0]);
        assert_eq!(p, [12.0, 7.0, 0.0, 1.0]);
    }
}
