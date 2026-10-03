//! Per-frame property expressions (Rhai).
//!
//! An expression is evaluated every frame for the property it is attached to. Scope variables:
//! `value` (keyframed value: number or array), `t` (clip-local seconds), `time` (composition
//! seconds), `duration`, `progress` (t / duration), `fps`, `frame`, `width`, `height`, `bpm`.
//!
//! Functions (see `EXPR_REFERENCE`): beat sync (`pulse`, `beat_phase`, `on_downbeat`...),
//! audio reactivity (`bass`, `treble`, `level`, `onset`...), noise (`wiggle`, `noise`...),
//! easing (`ease`, `tween`), math helpers and `var(name)` for project variables.

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use edits_audio::Envelope;
use edits_core::{Easing, Evaluator, Value};
use parking_lot::Mutex;
use rhai::{AST, Array, Dynamic, Engine, Scope};

/// Timing data shared by all expressions of a frame.
#[derive(Clone, Default, Debug)]
pub struct TimingData {
    pub bpm: f64,
    pub beats: Vec<f64>,
    pub downbeats: Vec<f64>,
    pub drops: Vec<f64>,
    pub accents: Vec<f64>,
    /// Envelope of the timing source (sampled at `t - envelope_offset`).
    pub envelope: Option<Arc<Envelope>>,
    pub envelope_offset: f64,
}

impl TimingData {
    fn last_before(list: &[f64], t: f64) -> Option<(usize, f64)> {
        let i = list.partition_point(|b| *b <= t + 1e-9);
        if i == 0 { None } else { Some((i - 1, list[i - 1])) }
    }
    pub fn since(list: &[f64], t: f64) -> f64 {
        Self::last_before(list, t).map(|(_, b)| t - b).unwrap_or(1e6)
    }
    pub fn beat_len(&self) -> f64 {
        if self.bpm > 0.0 {
            60.0 / self.bpm
        } else if self.beats.len() > 1 {
            (self.beats[self.beats.len() - 1] - self.beats[0]) / (self.beats.len() - 1) as f64
        } else {
            0.5
        }
    }
    pub fn beat_phase(&self, t: f64) -> f64 {
        match Self::last_before(&self.beats, t) {
            Some((i, b)) => {
                let next = self.beats.get(i + 1).copied().unwrap_or(b + self.beat_len());
                ((t - b) / (next - b).max(1e-6)).clamp(0.0, 1.0)
            }
            None => 0.0,
        }
    }
    pub fn bands(&self, t: f64) -> [f32; 4] {
        self.envelope.as_ref().map(|e| e.bands_at(t - self.envelope_offset)).unwrap_or([0.0; 4])
    }
    pub fn level(&self, t: f64) -> f32 {
        self.envelope.as_ref().map(|e| e.level_at(t - self.envelope_offset)).unwrap_or(0.0)
    }
    pub fn onset(&self, t: f64) -> f32 {
        self.envelope.as_ref().map(|e| e.onset_at(t - self.envelope_offset)).unwrap_or(0.0)
    }
}

/// Context for evaluating one clip's expressions.
#[derive(Clone)]
pub struct ExprCtx {
    pub comp_time: f64,
    /// Root timeline time (beats and audio are looked up here).
    pub root_time: f64,
    pub clip_start: f64,
    pub duration: f64,
    pub fps: f64,
    pub width: f64,
    pub height: f64,
    pub seed: u64,
    pub timing: Arc<TimingData>,
    pub variables: Arc<indexmap::IndexMap<String, Value>>,
}

thread_local! {
    static CTX: RefCell<Option<(ExprCtx, f64)>> = const { RefCell::new(None) };
}

fn with_ctx<R>(f: impl FnOnce(&ExprCtx, f64) -> R, default: R) -> R {
    CTX.with(|c| match &*c.borrow() {
        Some((ctx, t)) => f(ctx, *t),
        None => default,
    })
}

fn hash01(a: u64, b: u64) -> f64 {
    let mut x = a.wrapping_mul(0x9E3779B97F4A7C15) ^ b.wrapping_add(0x632BE59BD9B4E019).wrapping_mul(0xC2B2AE3D27D4EB4F);
    x ^= x >> 31;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// Smooth 1D value noise in -1..1.
pub fn noise1(x: f64, seed: u64) -> f64 {
    let i = x.floor();
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash01(i as i64 as u64, seed) * 2.0 - 1.0;
    let b = hash01((i as i64 + 1) as u64, seed) * 2.0 - 1.0;
    a + (b - a) * u
}

/// Fractal noise (3 octaves), roughly -1..1.
pub fn fbm1(x: f64, seed: u64) -> f64 {
    noise1(x, seed) * 0.57 + noise1(x * 2.1 + 7.3, seed ^ 0x55) * 0.29 + noise1(x * 4.3 + 1.7, seed ^ 0xAA) * 0.14
}

fn easing_by_name(name: &str) -> Easing {
    serde_json::from_value(serde_json::Value::String(name.to_string())).unwrap_or(Easing::Linear)
}

pub fn to_value(d: &Dynamic) -> Option<Value> {
    if let Some(f) = d.clone().try_cast::<f64>() {
        return Some(Value::Num(f));
    }
    if let Some(i) = d.clone().try_cast::<i64>() {
        return Some(Value::Num(i as f64));
    }
    if let Some(b) = d.clone().try_cast::<bool>() {
        return Some(Value::Bool(b));
    }
    if let Some(a) = d.clone().try_cast::<Array>() {
        let v: Option<Vec<f64>> = a.iter().map(|x| x.as_float().ok().or_else(|| x.as_int().ok().map(|i| i as f64))).collect();
        return v.map(Value::Vec);
    }
    if d.is_string() {
        return Some(Value::Str(d.clone().into_string().ok()?));
    }
    None
}

pub fn from_value(v: &Value) -> Dynamic {
    match v {
        Value::Num(n) => Dynamic::from_float(*n),
        Value::Bool(b) => Dynamic::from_bool(*b),
        Value::Vec(a) => Dynamic::from_array(a.iter().map(|x| Dynamic::from_float(*x)).collect()),
        Value::Str(s) => Dynamic::from(s.clone()),
    }
}

fn num(d: &Dynamic) -> f64 {
    d.as_float().ok().or_else(|| d.as_int().ok().map(|i| i as f64)).unwrap_or(0.0)
}

/// Register the math / noise / easing helpers shared by expressions and scripts.
pub fn register_common(engine: &mut Engine) {
    engine.register_fn("lerp", |a: f64, b: f64, t: f64| a + (b - a) * t);
    engine.register_fn("clamp", |x: f64, a: f64, b: f64| x.clamp(a.min(b), a.max(b)));
    engine.register_fn("smoothstep", |a: f64, b: f64, x: f64| {
        let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    });
    engine.register_fn("remap", |x: f64, a: f64, b: f64, c: f64, d: f64| c + (x - a) * (d - c) / (b - a));
    engine.register_fn("fract", |x: f64| x - x.floor());
    engine.register_fn("max", |a: f64, b: f64| a.max(b));
    engine.register_fn("min", |a: f64, b: f64| a.min(b));
    engine.register_fn("pow", |a: f64, b: f64| a.powf(b));
    engine.register_fn("fmod", |x: f64, y: f64| x.rem_euclid(y));
    engine.register_fn("sign", |x: f64| x.signum());
    engine.register_fn("deg", |x: f64| x.to_degrees());
    engine.register_fn("rad", |x: f64| x.to_radians());
    engine.register_fn("noise", |x: f64| noise1(x, 0));
    engine.register_fn("noise", |x: f64, seed: i64| noise1(x, seed as u64));
    engine.register_fn("fbm", |x: f64| fbm1(x, 0));
    engine.register_fn("hash", |x: f64| hash01(x.to_bits(), 1));
    engine.register_fn("random", |seed: i64| hash01(seed as u64, 99));
    engine.register_fn("random", |seed: f64| hash01(seed.to_bits(), 99));
    engine.register_fn("random_range", |a: f64, b: f64, seed: i64| a + (b - a) * hash01(seed as u64, 77));
    engine.register_fn("ease", |name: &str, x: f64| easing_by_name(name).apply(x, 1.0));
    engine.register_fn("ease", |x: f64| Easing::EaseInOutCubic.apply(x, 1.0));
    engine.register_fn("vec2", |x: f64, y: f64| -> Array { vec![Dynamic::from_float(x), Dynamic::from_float(y)] });
    engine.register_fn("vec3", |x: f64, y: f64, z: f64| -> Array { vec![Dynamic::from_float(x), Dynamic::from_float(y), Dynamic::from_float(z)] });
    engine.register_fn("rgba", |r: f64, g: f64, b: f64, a: f64| -> Array {
        vec![Dynamic::from_float(r), Dynamic::from_float(g), Dynamic::from_float(b), Dynamic::from_float(a)]
    });
    engine.register_fn("hsv", |h: f64, s: f64, v: f64| -> Array {
        let h = h.rem_euclid(360.0) / 60.0;
        let c = v * s;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let (r, g, b) = match h as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = v - c;
        vec![Dynamic::from_float(r + m), Dynamic::from_float(g + m), Dynamic::from_float(b + m), Dynamic::from_float(1.0)]
    });
    // vector math on arrays
    engine.register_fn("add", |a: Array, b: Array| -> Array { a.iter().zip(b.iter()).map(|(x, y)| Dynamic::from_float(num(x) + num(y))).collect() });
    engine.register_fn("mul", |a: Array, k: f64| -> Array { a.iter().map(|x| Dynamic::from_float(num(x) * k)).collect() });
    engine.register_fn("mix", |a: Array, b: Array, t: f64| -> Array {
        a.iter().zip(b.iter()).map(|(x, y)| Dynamic::from_float(num(x) + (num(y) - num(x)) * t)).collect()
    });
    engine.register_fn("mix", |a: f64, b: f64, t: f64| a + (b - a) * t);
}

fn register_vector_ops(engine: &mut Engine, override_add: bool) {
    let map1 = |a: &Array, f: &dyn Fn(f64) -> f64| -> Array { a.iter().map(|x| Dynamic::from_float(f(num(x)))).collect() };
    let zip = |a: &Array, b: &Array, f: &dyn Fn(f64, f64) -> f64| -> Array {
        let n = a.len().max(b.len());
        (0..n)
            .map(|i| {
                let x = a.get(i).or(a.last()).map(num).unwrap_or(0.0);
                let y = b.get(i).or(b.last()).map(num).unwrap_or(0.0);
                Dynamic::from_float(f(x, y))
            })
            .collect()
    };
    engine.register_fn("*", move |a: Array, k: f64| map1(&a, &|x| x * k));
    engine.register_fn("*", move |k: f64, a: Array| map1(&a, &|x| x * k));
    engine.register_fn("*", move |a: Array, k: i64| map1(&a, &|x| x * k as f64));
    engine.register_fn("*", move |k: i64, a: Array| map1(&a, &|x| x * k as f64));
    engine.register_fn("/", move |a: Array, k: f64| map1(&a, &|x| x / k));
    engine.register_fn("/", move |a: Array, k: i64| map1(&a, &|x| x / k as f64));
    engine.register_fn("*", move |a: Array, b: Array| zip(&a, &b, &|x, y| x * y));
    engine.register_fn("/", move |a: Array, b: Array| zip(&a, &b, &|x, y| x / y));
    engine.register_fn("-", move |a: Array, b: Array| zip(&a, &b, &|x, y| x - y));
    engine.register_fn("-", move |a: Array| map1(&a, &|x| -x));
    engine.register_fn("+", move |a: Array, k: f64| map1(&a, &|x| x + k));
    engine.register_fn("-", move |a: Array, k: f64| map1(&a, &|x| x - k));
    if override_add {
        engine.register_fn("+", move |a: Array, b: Array| zip(&a, &b, &|x, y| x + y));
    }
}

fn register_expr_fns(engine: &mut Engine) {
    // ---- beat sync ----
    engine.register_fn("beat", || with_ctx(|c, _| TimingData::last_before(&c.timing.beats, c.root_time).map(|(i, _)| i as f64).unwrap_or(-1.0), -1.0));
    engine.register_fn("beat_phase", || with_ctx(|c, _| c.timing.beat_phase(c.root_time), 0.0));
    engine.register_fn("since_beat", || with_ctx(|c, _| TimingData::since(&c.timing.beats, c.root_time), 1e6));
    engine.register_fn("since_downbeat", || with_ctx(|c, _| TimingData::since(&c.timing.downbeats, c.root_time), 1e6));
    engine.register_fn("since_drop", || with_ctx(|c, _| TimingData::since(&c.timing.drops, c.root_time), 1e6));
    engine.register_fn("to_next_beat", || {
        with_ctx(
            |c, _| {
                let i = c.timing.beats.partition_point(|b| *b <= c.root_time);
                c.timing.beats.get(i).map(|b| b - c.root_time).unwrap_or(1e6)
            },
            1e6,
        )
    });
    engine.register_fn("pulse", |decay: f64| with_ctx(|c, _| (-TimingData::since(&c.timing.beats, c.root_time) * decay).exp(), 0.0));
    engine.register_fn("pulse", || with_ctx(|c, _| (-TimingData::since(&c.timing.beats, c.root_time) * 8.0).exp(), 0.0));
    engine.register_fn("downbeat_pulse", |decay: f64| with_ctx(|c, _| (-TimingData::since(&c.timing.downbeats, c.root_time) * decay).exp(), 0.0));
    engine.register_fn("drop_pulse", |decay: f64| with_ctx(|c, _| (-TimingData::since(&c.timing.drops, c.root_time) * decay).exp(), 0.0));
    engine.register_fn("accent_pulse", |decay: f64| with_ctx(|c, _| (-TimingData::since(&c.timing.accents, c.root_time) * decay).exp(), 0.0));
    engine.register_fn("pulse_every", |n: i64, decay: f64| {
        with_ctx(
            |c, _| match TimingData::last_before(&c.timing.beats, c.root_time) {
                Some((i, b)) if n > 0 && i as i64 % n == 0 => (-(c.root_time - b) * decay).exp(),
                _ => 0.0,
            },
            0.0,
        )
    });
    engine.register_fn("beat_in_bar", || {
        with_ctx(
            |c, _| {
                let since_db = TimingData::since(&c.timing.downbeats, c.root_time);
                (since_db / c.timing.beat_len()).floor().clamp(0.0, 3.0)
            },
            0.0,
        )
    });
    engine.register_fn("bar", || with_ctx(|c, _| TimingData::last_before(&c.timing.downbeats, c.root_time).map(|(i, _)| i as f64).unwrap_or(-1.0), -1.0));
    engine.register_fn("beats_to_sec", |b: f64| with_ctx(|c, _| b * c.timing.beat_len(), b * 0.5));
    engine.register_fn("beat_len", || with_ctx(|c, _| c.timing.beat_len(), 0.5));
    engine.register_fn("in_drop", |window: f64| with_ctx(|c, _| TimingData::since(&c.timing.drops, c.root_time) < window, false));
    // alternate a value on each beat: returns 0/1 toggling every beat
    engine.register_fn("beat_toggle", || {
        with_ctx(|c, _| TimingData::last_before(&c.timing.beats, c.root_time).map(|(i, _)| (i % 2) as f64).unwrap_or(0.0), 0.0)
    });
    // ---- audio ----
    engine.register_fn("bass", || with_ctx(|c, _| c.timing.bands(c.root_time)[0] as f64, 0.0));
    engine.register_fn("low_mid", || with_ctx(|c, _| c.timing.bands(c.root_time)[1] as f64, 0.0));
    engine.register_fn("high_mid", || with_ctx(|c, _| c.timing.bands(c.root_time)[2] as f64, 0.0));
    engine.register_fn("treble", || with_ctx(|c, _| c.timing.bands(c.root_time)[3] as f64, 0.0));
    engine.register_fn("level", || with_ctx(|c, _| c.timing.level(c.root_time) as f64, 0.0));
    engine.register_fn("onset", || with_ctx(|c, _| c.timing.onset(c.root_time) as f64, 0.0));
    engine.register_fn("bass_at", |dt: f64| with_ctx(|c, _| c.timing.bands(c.root_time + dt)[0] as f64, 0.0));
    // ---- noise in time ----
    engine.register_fn("wiggle", |freq: f64, amp: f64| with_ctx(|c, t| fbm1(t * freq, c.seed) * amp, 0.0));
    engine.register_fn("wiggle", |freq: f64, amp: f64, seed: i64| with_ctx(|c, t| fbm1(t * freq, c.seed ^ seed as u64) * amp, 0.0));
    engine.register_fn("wiggle2", |freq: f64, amp: f64| -> Array {
        with_ctx(
            |c, t| vec![Dynamic::from_float(fbm1(t * freq, c.seed) * amp), Dynamic::from_float(fbm1(t * freq, c.seed ^ 0xBEEF) * amp)],
            vec![Dynamic::from_float(0.0), Dynamic::from_float(0.0)],
        )
    });
    engine.register_fn("jitter", |rate: f64, amp: f64| with_ctx(|c, t| (hash01((t * rate).floor() as i64 as u64, c.seed) * 2.0 - 1.0) * amp, 0.0));
    engine.register_fn("flicker", |rate: f64| with_ctx(|c, t| hash01((t * rate).floor() as i64 as u64, c.seed ^ 3), 0.0));
    engine.register_fn("rand_clip", || with_ctx(|c, _| hash01(c.seed, 5), 0.0));
    // ---- time helpers ----
    engine.register_fn("step_time", |fps: f64| with_ctx(|_, t| (t * fps).floor() / fps.max(1e-6), 0.0));
    engine.register_fn("loop_time", |period: f64| with_ctx(|_, t| t.rem_euclid(period.max(1e-6)), 0.0));
    engine.register_fn("tween", |t0: f64, t1: f64, v0: f64, v1: f64, ease: &str| {
        with_ctx(|_, t| v0 + (v1 - v0) * easing_by_name(ease).apply(((t - t0) / (t1 - t0)).clamp(0.0, 1.0), t1 - t0), v0)
    });
    engine.register_fn("tween", |t0: f64, t1: f64, v0: f64, v1: f64| with_ctx(|_, t| v0 + (v1 - v0) * ((t - t0) / (t1 - t0)).clamp(0.0, 1.0), v0));
    engine.register_fn("var", |name: &str| -> Dynamic {
        with_ctx(|c, _| c.variables.get(name).map(from_value).unwrap_or(Dynamic::UNIT), Dynamic::UNIT)
    });
    engine.register_fn("var", |name: &str, default: f64| -> Dynamic {
        with_ctx(|c, _| c.variables.get(name).map(from_value).unwrap_or(Dynamic::from_float(default)), Dynamic::from_float(default))
    });
}

/// Human/agent-readable reference of expression features.
pub const EXPR_REFERENCE: &str = r#"Expressions are Rhai code evaluated every frame; the last value is the property value.
Variables: value (keyframed value; number or [x,y]/[r,g,b,a] array), t (clip-local s), time (comp s),
  duration, progress (t/duration), fps, frame, width, height, bpm.
Beat sync (needs project timing; run analyze_audio with apply=true):
  pulse(decay=8) -> 1 on each beat decaying exponentially; downbeat_pulse(d); drop_pulse(d); accent_pulse(d)
  pulse_every(n, decay); beat(); bar(); beat_in_bar() 0..3; beat_phase() 0..1; since_beat(); since_downbeat();
  since_drop(); to_next_beat(); beat_toggle() 0/1; beat_len(); beats_to_sec(b); in_drop(window_s)
Audio reactive (0..~1, from the timing source track): bass(), low_mid(), high_mid(), treble(), level(), onset(), bass_at(dt)
Noise & randomness: wiggle(freq, amp[, seed]); wiggle2(freq, amp) -> [x,y]; jitter(rate, amp); flicker(rate) 0..1;
  noise(x[, seed]) -1..1; fbm(x); random(seed) 0..1; random_range(a,b,seed); rand_clip() (stable per clip)
Time: step_time(fps) (choppy 'on twos'); loop_time(period); tween(t0, t1, v0, v1[, ease])
Math: lerp, mix, clamp, smoothstep, remap, fract, fmod, sign, deg, rad, sin/cos/abs/floor/sqrt/... (Rhai builtins), PI
Easing: ease(name, x) e.g. ease("ease_out_expo", progress)
Vectors/colors: arrays support component-wise math: value * 1.2, value + [10.0, 0.0], [a,b] * [c,d];
  vec2(x,y), vec3, rgba(r,g,b,a), hsv(h,s,v), add(a,b), mul(a,k), mix(a,b,t)
Project variables: var("name"[, default])
Examples:
  scale:    value * (1.0 + 0.15 * pulse(10.0))            // zoom punch on every beat
  rotation: wiggle(3.0, 4.0)                              // handheld sway
  opacity:  if beat_toggle() == 0.0 { 1.0 } else { 0.4 }  // strobe per beat
  position: add(value, wiggle2(8.0, 20.0 * bass()))       // bass-driven shake
  effect param 'amount' of rgb_split: 30.0 * pulse(12.0) + 5.0 * treble()
"#;

pub struct ExprEngine {
    engine: Engine,
    cache: Mutex<HashMap<u64, std::result::Result<Arc<AST>, String>>>,
    library: Mutex<(u64, String)>,
}

impl Default for ExprEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ExprEngine {
    pub fn new() -> ExprEngine {
        let mut engine = Engine::new();
        engine.set_max_operations(200_000);
        engine.set_max_expr_depths(64, 32);
        engine.set_max_call_levels(32);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(10_000);
        register_common(&mut engine);
        register_vector_ops(&mut engine, true);
        register_expr_fns(&mut engine);
        ExprEngine { engine, cache: Mutex::new(HashMap::new()), library: Mutex::new((0, String::new())) }
    }

    /// Set the project-wide function library (prepended to every expression).
    pub fn set_library(&self, lib: &str) {
        let h = blake3::hash(lib.as_bytes());
        let key = u64::from_le_bytes(h.as_bytes()[..8].try_into().unwrap());
        let mut l = self.library.lock();
        if l.0 != key {
            *l = (key, lib.to_string());
            self.cache.lock().clear();
        }
    }

    pub fn compile(&self, expr: &str) -> std::result::Result<Arc<AST>, String> {
        let key = {
            let h = blake3::hash(expr.as_bytes());
            u64::from_le_bytes(h.as_bytes()[..8].try_into().unwrap())
        };
        if let Some(r) = self.cache.lock().get(&key) {
            return r.clone();
        }
        let lib = self.library.lock().1.clone();
        let src = if lib.is_empty() { expr.to_string() } else { format!("{lib}\n{expr}") };
        let r = self.engine.compile(&src).map(Arc::new).map_err(|e| e.to_string());
        self.cache.lock().insert(key, r.clone());
        r
    }

    /// Evaluate an expression with a context. Errors are returned as strings.
    pub fn eval(&self, expr: &str, ctx: &ExprCtx, t: f64, value: &Value) -> std::result::Result<Value, String> {
        let ast = self.compile(expr)?;
        let mut scope = Scope::new();
        scope.push_constant("value", from_value(value));
        scope.push_constant("t", t);
        scope.push_constant("time", ctx.comp_time);
        scope.push_constant("duration", ctx.duration);
        scope.push_constant("progress", if ctx.duration > 0.0 { (t / ctx.duration).clamp(0.0, 1.0) } else { 0.0 });
        scope.push_constant("fps", ctx.fps);
        scope.push_constant("frame", (ctx.comp_time * ctx.fps).floor());
        scope.push_constant("width", ctx.width);
        scope.push_constant("height", ctx.height);
        scope.push_constant("bpm", ctx.timing.bpm);
        CTX.with(|c| *c.borrow_mut() = Some((ctx.clone(), t)));
        let r = self.engine.eval_ast_with_scope::<Dynamic>(&mut scope, &ast);
        CTX.with(|c| *c.borrow_mut() = None);
        let d = r.map_err(|e| e.to_string())?;
        to_value(&d).ok_or_else(|| format!("expression returned unsupported type {}", d.type_name()))
    }
}

/// `Evaluator` bound to one clip's context.
pub struct ClipEval<'a> {
    pub engine: &'a ExprEngine,
    pub ctx: ExprCtx,
    pub errors: std::rc::Rc<RefCell<Vec<String>>>,
}

impl Evaluator for ClipEval<'_> {
    fn eval_expr(&self, expr: &str, t: f64, value: &Value) -> Option<Value> {
        match self.engine.eval(expr, &self.ctx, t, value) {
            Ok(v) => Some(v),
            Err(e) => {
                let mut errs = self.errors.borrow_mut();
                if errs.len() < 20 {
                    errs.push(format!("expression `{expr}`: {e}"));
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ExprCtx {
        ExprCtx {
            comp_time: 1.05,
            root_time: 1.05,
            clip_start: 0.0,
            duration: 2.0,
            fps: 24.0,
            width: 1920.0,
            height: 1080.0,
            seed: 1,
            timing: Arc::new(TimingData { bpm: 120.0, beats: vec![0.0, 0.5, 1.0, 1.5], downbeats: vec![0.0], ..Default::default() }),
            variables: Arc::new(indexmap::IndexMap::new()),
        }
    }

    #[test]
    fn evaluates() {
        let e = ExprEngine::new();
        let c = ctx();
        let v = e.eval("value * 2.0", &c, 1.05, &Value::Num(3.0)).unwrap();
        assert_eq!(v, Value::Num(6.0));
        let p = e.eval("pulse(10.0)", &c, 1.05, &Value::Num(0.0)).unwrap().as_f64().unwrap();
        assert!((p - (-0.5f64).exp()).abs() < 1e-6, "{p}");
        let arr = e.eval("add(value, vec2(1.0, 2.0))", &c, 1.05, &Value::Vec(vec![1.0, 1.0])).unwrap();
        assert_eq!(arr, Value::Vec(vec![2.0, 3.0]));
        assert_eq!(e.eval("value * 2.0 + [1.0, 0.0]", &c, 1.05, &Value::Vec(vec![1.0, 1.0])).unwrap(), Value::Vec(vec![3.0, 2.0]));
        assert!(e.eval("nope(", &c, 0.0, &Value::Num(0.0)).is_err());
        let w = e.eval("wiggle(2.0, 10.0)", &c, 1.0, &Value::Num(0.0)).unwrap().as_f64().unwrap();
        assert!(w.abs() <= 10.0);
        e.set_library("fn double(x) { x * 2.0 }");
        assert_eq!(e.eval("double(value)", &c, 0.0, &Value::Num(2.0)).unwrap(), Value::Num(4.0));
    }
}
