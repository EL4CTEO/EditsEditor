//! Colors. Stored as straight-alpha sRGB in `[0, 1]`.
//!
//! Accepted JSON forms:
//! - `"#rgb"`, `"#rgba"`, `"#rrggbb"`, `"#rrggbbaa"`
//! - CSS-like names (`"white"`, `"hotpink"`, `"transparent"`, ...)
//! - `"rgb(255, 0, 128)"` / `"rgba(255, 0, 128, 0.5)"` / `"hsl(320, 100%, 50%)"`
//! - `[r, g, b]` / `[r, g, b, a]` with components in 0..1 (or 0..255 if any > 1)

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub [f32; 4]);

impl Default for Color {
    fn default() -> Self {
        Color::WHITE
    }
}

impl Color {
    pub const WHITE: Color = Color([1.0, 1.0, 1.0, 1.0]);
    pub const BLACK: Color = Color([0.0, 0.0, 0.0, 1.0]);
    pub const TRANSPARENT: Color = Color([0.0, 0.0, 0.0, 0.0]);

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color([r, g, b, a])
    }

    /// Convert to linear-light, premultiplied RGBA (the renderer's working space).
    pub fn to_linear_premul(&self) -> [f32; 4] {
        let [r, g, b, a] = self.0;
        [srgb_to_linear(r) * a, srgb_to_linear(g) * a, srgb_to_linear(b) * a, a]
    }

    pub fn to_hex(&self) -> String {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        let [r, g, b, a] = self.0;
        if c(a) == 255 {
            format!("#{:02x}{:02x}{:02x}", c(r), c(g), c(b))
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", c(r), c(g), c(b), c(a))
        }
    }

    pub fn parse(s: &str) -> Option<Color> {
        let s = s.trim();
        if let Some(hex) = s.strip_prefix('#') {
            return parse_hex(hex);
        }
        let lower = s.to_ascii_lowercase();
        if let Some(args) = func_args(&lower, "rgba").or_else(|| func_args(&lower, "rgb")) {
            let v = parse_numbers(args)?;
            if v.len() < 3 {
                return None;
            }
            let a = v.get(3).copied().unwrap_or(1.0);
            return Some(Color([
                (v[0] / 255.0) as f32,
                (v[1] / 255.0) as f32,
                (v[2] / 255.0) as f32,
                a as f32,
            ]));
        }
        if let Some(args) = func_args(&lower, "hsla").or_else(|| func_args(&lower, "hsl")) {
            let v = parse_numbers(args)?;
            if v.len() < 3 {
                return None;
            }
            let (r, g, b) = hsl_to_rgb(v[0], v[1] / 100.0, v[2] / 100.0);
            let a = v.get(3).copied().unwrap_or(1.0);
            return Some(Color([r as f32, g as f32, b as f32, a as f32]));
        }
        named(&lower).and_then(parse_hex)
    }

    pub fn from_slice(v: &[f64]) -> Option<Color> {
        if v.len() < 3 {
            return None;
        }
        let scale = if v.iter().take(3).any(|c| *c > 1.0 + 1e-6) && v.iter().take(3).all(|c| *c >= 0.0 && *c <= 255.0 && c.fract() == 0.0) { 255.0 } else { 1.0 };
        let a = v.get(3).copied().unwrap_or(1.0);
        let a = if scale == 255.0 && a > 1.0 { a / 255.0 } else { a };
        Some(Color([
            (v[0] / scale) as f32,
            (v[1] / scale) as f32,
            (v[2] / scale) as f32,
            a as f32,
        ]))
    }

    pub fn lerp(&self, o: &Color, t: f64) -> Color {
        let t = t as f32;
        let mut r = [0.0; 4];
        for (i, v) in r.iter_mut().enumerate() {
            *v = self.0[i] + (o.0[i] - self.0[i]) * t;
        }
        Color(r)
    }
}

pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 }
}

fn func_args<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    s.strip_prefix(name)?.trim().strip_prefix('(')?.strip_suffix(')')
}

fn parse_numbers(args: &str) -> Option<Vec<f64>> {
    args.split([',', ' ', '/'])
        .filter(|p| !p.is_empty())
        .map(|p| {
            let p = p.trim();
            if let Some(pct) = p.strip_suffix('%') {
                pct.parse::<f64>().ok()
            } else {
                p.trim_end_matches("deg").parse::<f64>().ok()
            }
        })
        .collect()
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let h = h.rem_euclid(360.0) / 360.0;
    if s <= 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hue = |mut t: f64| {
        t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (hue(h + 1.0 / 3.0), hue(h), hue(h - 1.0 / 3.0))
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    let digits: Vec<u8> = hex
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<Vec<_>>>()?;
    let (r, g, b, a) = match digits.len() {
        3 => (digits[0] * 17, digits[1] * 17, digits[2] * 17, 255),
        4 => (digits[0] * 17, digits[1] * 17, digits[2] * 17, digits[3] * 17),
        6 => (digits[0] * 16 + digits[1], digits[2] * 16 + digits[3], digits[4] * 16 + digits[5], 255),
        8 => (
            digits[0] * 16 + digits[1],
            digits[2] * 16 + digits[3],
            digits[4] * 16 + digits[5],
            digits[6] * 16 + digits[7],
        ),
        _ => return None,
    };
    Some(Color([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]))
}

fn named(name: &str) -> Option<&'static str> {
    Some(match name {
        "transparent" | "none" => "00000000",
        "white" => "ffffff",
        "black" => "000000",
        "red" => "ff0000",
        "green" => "00ff00",
        "blue" => "0000ff",
        "yellow" => "ffff00",
        "cyan" | "aqua" => "00ffff",
        "magenta" | "fuchsia" => "ff00ff",
        "orange" => "ff8800",
        "purple" => "8000ff",
        "violet" => "ee82ee",
        "pink" => "ffc0cb",
        "hotpink" => "ff69b4",
        "deeppink" => "ff1493",
        "gray" | "grey" => "808080",
        "silver" => "c0c0c0",
        "gold" => "ffd700",
        "crimson" => "dc143c",
        "teal" => "008080",
        "navy" => "000080",
        "lime" => "32ff32",
        "indigo" => "4b0082",
        "skyblue" => "87ceeb",
        "coral" => "ff7f50",
        "salmon" => "fa8072",
        "turquoise" => "40e0d0",
        "lavender" => "e6e6fa",
        "mint" => "98ff98",
        "neonpink" => "ff10f0",
        "neongreen" => "39ff14",
        "neonblue" => "1f51ff",
        "electricpurple" => "bf00ff",
        "sakura" => "ffb7c5",
        _ => return None,
    })
}

impl Color {
    /// True if every channel is exactly representable in 8 bits (so hex is lossless).
    pub fn is_8bit(&self) -> bool {
        self.0.iter().all(|c| {
            let v = c * 255.0;
            (v - v.round()).abs() < 1e-4 && (0.0..=1.0).contains(c)
        })
    }
}

impl Serialize for Color {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.is_8bit() {
            s.serialize_str(&self.to_hex())
        } else {
            self.0.serialize(s)
        }
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            S(String),
            A(Vec<f64>),
        }
        match Repr::deserialize(d)? {
            Repr::S(s) => Color::parse(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid color '{s}'"))),
            Repr::A(a) => Color::from_slice(&a).ok_or_else(|| serde::de::Error::custom("color array needs 3 or 4 numbers")),
        }
    }
}

impl JsonSchema for Color {
    fn schema_name() -> Cow<'static, str> {
        "Color".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "Color: '#rrggbb[aa]', CSS name, 'rgb()/rgba()/hsl()', or [r,g,b(,a)] in 0..1.",
            "oneOf": [
                { "type": "string" },
                { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 4 }
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_forms() {
        assert_eq!(Color::parse("#fff").unwrap(), Color::WHITE);
        assert_eq!(Color::parse("black").unwrap(), Color::BLACK);
        let c = Color::parse("rgba(255, 0, 0, 0.5)").unwrap();
        assert_eq!(c.0, [1.0, 0.0, 0.0, 0.5]);
        let h = Color::parse("hsl(120, 100%, 50%)").unwrap();
        assert!((h.0[1] - 1.0).abs() < 1e-6 && h.0[0].abs() < 1e-6);
        assert_eq!(Color::parse("#ff000080").unwrap().to_hex(), "#ff000080");
        let c = Color([0.1, 0.0, 0.2, 1.0]);
        let back: Color = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(c, back);
    }
}
