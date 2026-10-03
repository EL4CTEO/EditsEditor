//! Easing curves for keyframe interpolation.
//!
//! JSON forms: a name (`"ease_out_expo"`), or an object:
//! `{"bezier": [x1, y1, x2, y2]}`, `{"steps": 4}`,
//! `{"spring": {"stiffness": 180, "damping": 12}}`, `{"elastic": {"amplitude": 1, "period": 0.3}}`,
//! `{"back": 2.5}` (overshoot amount).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    #[default]
    Linear,
    /// Holds the current value until the next keyframe (jump cut on values).
    Hold,
    /// CSS `ease` (0.25, 0.1, 0.25, 1).
    Ease,
    /// Alias for `ease_in_cubic`.
    EaseIn,
    /// Alias for `ease_out_cubic`.
    EaseOut,
    /// Alias for `ease_in_out_cubic`.
    EaseInOut,
    EaseInSine,
    EaseOutSine,
    EaseInOutSine,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,
    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,
    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,
    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,
    EaseInBack,
    EaseOutBack,
    EaseInOutBack,
    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,
    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,
    Smoothstep,
    Smootherstep,
    /// Very sharp in-out curve used for velocity edits / whip moves (bezier 0.9,0,0.1,1).
    Whip,
    /// Fast start that settles hard — great for zoom punches (bezier 0.05,0.7,0.1,1).
    Punch,
    /// Smooth anime-edit style ease (bezier 0.7,0,0.2,1).
    Snap,
    /// Slow-fast-slow, softer than whip (bezier 0.65,0,0.35,1).
    Glide,
    /// Custom cubic bezier like CSS `cubic-bezier(x1, y1, x2, y2)`.
    Bezier([f64; 4]),
    /// Stepped interpolation with N steps.
    Steps(u32),
    /// Overshoot ("back") with custom overshoot amount (default style is 1.70158).
    Back(f64),
    /// Damped spring. Interpreted in seconds over the keyframe segment.
    Spring(SpringParams),
    /// Elastic with custom amplitude and period.
    Elastic(ElasticParams),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpringParams {
    #[serde(default = "default_stiffness")]
    pub stiffness: f64,
    #[serde(default = "default_damping")]
    pub damping: f64,
    #[serde(default = "one")]
    pub mass: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ElasticParams {
    #[serde(default = "one")]
    pub amplitude: f64,
    #[serde(default = "default_period")]
    pub period: f64,
}

fn default_stiffness() -> f64 {
    170.0
}
fn default_damping() -> f64 {
    12.0
}
fn one() -> f64 {
    1.0
}
fn default_period() -> f64 {
    0.3
}

impl Easing {
    /// All named easings (for discovery tools).
    pub const NAMES: &'static [&'static str] = &[
        "linear", "hold", "ease", "ease_in", "ease_out", "ease_in_out", "ease_in_sine", "ease_out_sine",
        "ease_in_out_sine", "ease_in_quad", "ease_out_quad", "ease_in_out_quad", "ease_in_cubic",
        "ease_out_cubic", "ease_in_out_cubic", "ease_in_quart", "ease_out_quart", "ease_in_out_quart",
        "ease_in_quint", "ease_out_quint", "ease_in_out_quint", "ease_in_expo", "ease_out_expo",
        "ease_in_out_expo", "ease_in_circ", "ease_out_circ", "ease_in_out_circ", "ease_in_back",
        "ease_out_back", "ease_in_out_back", "ease_in_elastic", "ease_out_elastic", "ease_in_out_elastic",
        "ease_in_bounce", "ease_out_bounce", "ease_in_out_bounce", "smoothstep", "smootherstep", "whip",
        "punch", "snap", "glide",
    ];

    /// Map normalized progress `t` in [0,1] to eased progress.
    /// `segment_seconds` is the duration of the keyframe segment (used by springs).
    pub fn apply(&self, t: f64, segment_seconds: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        use Easing::*;
        match self {
            Linear => t,
            Hold => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Ease => cubic_bezier(0.25, 0.1, 0.25, 1.0, t),
            EaseIn | EaseInCubic => t * t * t,
            EaseOut | EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            EaseInOut | EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            EaseInSine => 1.0 - (t * PI / 2.0).cos(),
            EaseOutSine => (t * PI / 2.0).sin(),
            EaseInOutSine => -((PI * t).cos() - 1.0) / 2.0,
            EaseInQuad => t * t,
            EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EaseInQuart => t.powi(4),
            EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t.powi(4)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            EaseInQuint => t.powi(5),
            EaseOutQuint => 1.0 - (1.0 - t).powi(5),
            EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t.powi(5)
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }
            EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    2f64.powf(10.0 * t - 10.0)
                }
            }
            EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2f64.powf(-10.0 * t)
                }
            }
            EaseInOutExpo => {
                if t == 0.0 || t == 1.0 {
                    t
                } else if t < 0.5 {
                    2f64.powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - 2f64.powf(-20.0 * t + 10.0)) / 2.0
                }
            }
            EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            EaseOutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
            EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }
            EaseInBack => back_in(t, 1.70158),
            EaseOutBack => 1.0 - back_in(1.0 - t, 1.70158),
            EaseInOutBack => {
                let c = 1.70158 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c + 1.0) * 2.0 * t - c)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c + 1.0) * (t * 2.0 - 2.0) + c) + 2.0) / 2.0
                }
            }
            Back(s) => 1.0 - back_in(1.0 - t, *s),
            EaseInElastic => 1.0 - elastic_out(1.0 - t, 1.0, 0.3),
            EaseOutElastic => elastic_out(t, 1.0, 0.3),
            EaseInOutElastic => {
                if t < 0.5 {
                    (1.0 - elastic_out(1.0 - 2.0 * t, 1.0, 0.45)) / 2.0
                } else {
                    (1.0 + elastic_out(2.0 * t - 1.0, 1.0, 0.45)) / 2.0
                }
            }
            Elastic(p) => elastic_out(t, p.amplitude, p.period),
            EaseInBounce => 1.0 - bounce_out(1.0 - t),
            EaseOutBounce => bounce_out(t),
            EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }
            Smoothstep => t * t * (3.0 - 2.0 * t),
            Smootherstep => t * t * t * (t * (t * 6.0 - 15.0) + 10.0),
            Whip => cubic_bezier(0.9, 0.0, 0.1, 1.0, t),
            Punch => cubic_bezier(0.05, 0.7, 0.1, 1.0, t),
            Snap => cubic_bezier(0.7, 0.0, 0.2, 1.0, t),
            Glide => cubic_bezier(0.65, 0.0, 0.35, 1.0, t),
            Bezier([x1, y1, x2, y2]) => cubic_bezier(*x1, *y1, *x2, *y2, t),
            Steps(n) => {
                let n = (*n).max(1) as f64;
                (t * n).floor().min(n) / n
            }
            Spring(p) => spring(t * segment_seconds.max(1e-6), p, segment_seconds.max(1e-6)),
        }
    }
}

fn back_in(t: f64, s: f64) -> f64 {
    t * t * ((s + 1.0) * t - s)
}

fn elastic_out(t: f64, amplitude: f64, period: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    let a = amplitude.max(1.0);
    let p = period.max(0.01);
    let s = p / (2.0 * PI) * (1.0 / a).asin();
    a * 2f64.powf(-10.0 * t) * ((t - s) * (2.0 * PI) / p).sin() + 1.0
}

fn bounce_out(t: f64) -> f64 {
    let n1 = 7.5625;
    let d1 = 2.75;
    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Damped harmonic oscillator response, forced to land exactly at 1 at the segment end.
fn spring(time: f64, p: &SpringParams, total: f64) -> f64 {
    let resp = |x: f64| {
        let m = p.mass.max(1e-4);
        let k = p.stiffness.max(1e-4);
        let c = p.damping.max(0.0);
        let w0 = (k / m).sqrt();
        let zeta = c / (2.0 * (k * m).sqrt());
        if zeta < 1.0 {
            let wd = w0 * (1.0 - zeta * zeta).sqrt();
            1.0 - (-zeta * w0 * x).exp() * ((wd * x).cos() + (zeta * w0 / wd) * (wd * x).sin())
        } else {
            1.0 - (-w0 * x).exp() * (1.0 + w0 * x)
        }
    };
    let end = resp(total);
    // blend residual error away over the final 15% so the segment lands on the target value
    let t = time / total;
    let fix = ((t - 0.85) / 0.15).clamp(0.0, 1.0);
    let v = resp(time);
    v + (1.0 - end) * fix * (time / total)
}

/// Solve a CSS-style cubic bezier for y given x.
pub fn cubic_bezier(x1: f64, y1: f64, x2: f64, y2: f64, x: f64) -> f64 {
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;
    let sample_x = |t: f64| ((ax * t + bx) * t + cx) * t;
    let sample_y = |t: f64| ((ay * t + by) * t + cy) * t;
    let deriv_x = |t: f64| (3.0 * ax * t + 2.0 * bx) * t + cx;

    // Newton-Raphson, then bisection fallback.
    let mut t = x;
    for _ in 0..8 {
        let err = sample_x(t) - x;
        if err.abs() < 1e-7 {
            return sample_y(t);
        }
        let d = deriv_x(t);
        if d.abs() < 1e-6 {
            break;
        }
        t -= err / d;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    t = x;
    for _ in 0..40 {
        let v = sample_x(t);
        if (v - x).abs() < 1e-7 {
            break;
        }
        if x > v {
            lo = t;
        } else {
            hi = t;
        }
        t = (lo + hi) / 2.0;
    }
    sample_y(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints() {
        for name in Easing::NAMES {
            let e: Easing = serde_json::from_value(serde_json::Value::String(name.to_string())).unwrap();
            if matches!(e, Easing::Hold) {
                continue;
            }
            assert!((e.apply(0.0, 1.0)).abs() < 1e-6, "{name} start");
            assert!((e.apply(1.0, 1.0) - 1.0).abs() < 1e-6, "{name} end");
        }
    }

    #[test]
    fn object_forms() {
        let e: Easing = serde_json::from_str(r#"{"bezier":[0.4,0,0.2,1]}"#).unwrap();
        assert!(e.apply(0.5, 1.0) > 0.5);
        let s: Easing = serde_json::from_str(r#"{"spring":{"stiffness":200}}"#).unwrap();
        assert!((s.apply(1.0, 0.5) - 1.0).abs() < 1e-6);
    }
}
