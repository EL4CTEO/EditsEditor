//! Rhai scripting for presets and agent scripts.
//!
//! Scripts see:
//! - `project`: a live handle with methods (`add_track`, `add_clip`, `add_effect`, `update`,
//!   `set`, `get`, `keyframe`, `remove`, `split`, `move_clip`, `duplicate`, `apply_preset`,
//!   `clip`, `clips`, `tracks`, `assets`, `asset`, `comp`, `set_duration`, `add_marker`, `json`).
//! - `clip` (clip presets): the clip as a mutable map — edit fields directly.
//! - `args`: preset params with defaults; `ctx`: timing/composition info.
//! - helpers: `fx`, `keys`, `expr`, `pulse_keys`, `add_fx`, `set_kf`, `animate`, `merge`,
//!   `get_path`, `beats`, `downbeats`, `drops`, `sections`, `accents`, `bpm`, `beats_between`,
//!   `nearest_beat`, `scenes`, `subtitles`, `rand`, `rand_int`, `choose`, `shuffle`, `log`.

use std::sync::Arc;

use edits_core::{EffectInstance, Easing, Project, Value, ops};
use edits_fx::{Library, PresetDef, PresetScope};
use parking_lot::Mutex;
use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Scope};
use serde_json::{Value as Json, json};

use crate::expr::{from_value, register_common, to_value};

/// Engine services available to scripts (slow operations are cached by the engine).
pub trait ScriptServices: Send + Sync {
    fn scenes(&self, project: &Project, asset: &str) -> Result<Json, String>;
    fn subtitles(&self, project: &Project, asset: &str) -> Result<Json, String>;
}

pub struct NoServices;
impl ScriptServices for NoServices {
    fn scenes(&self, _: &Project, _: &str) -> Result<Json, String> {
        Err("scene detection unavailable".into())
    }
    fn subtitles(&self, _: &Project, _: &str) -> Result<Json, String> {
        Err("subtitles unavailable".into())
    }
}

pub struct ScriptState {
    pub project: Project,
    pub lib: Arc<Library>,
    pub logs: Vec<String>,
    pub services: Arc<dyn ScriptServices>,
    pub depth: u32,
    pub rng: u64,
}

#[derive(Clone)]
pub struct ProjectHandle(pub Arc<Mutex<ScriptState>>);

pub struct ScriptOutcome {
    pub project: Project,
    pub logs: Vec<String>,
    pub result: Json,
}

type RResult<T> = Result<T, Box<EvalAltResult>>;

fn err<T>(msg: impl Into<String>) -> RResult<T> {
    Err(msg.into().into())
}

pub fn to_dyn(v: &Json) -> Dynamic {
    rhai::serde::to_dynamic(v).unwrap_or(Dynamic::UNIT)
}

pub fn from_dyn(d: &Dynamic) -> Json {
    rhai::serde::from_dynamic::<Json>(d).unwrap_or(Json::Null)
}

fn map_to_json(m: &Map) -> Json {
    from_dyn(&Dynamic::from_map(m.clone()))
}

fn f(d: &Dynamic) -> f64 {
    d.as_float().ok().or_else(|| d.as_int().ok().map(|i| i as f64)).unwrap_or(0.0)
}

fn next_rand(state: &mut u64) -> f64 {
    // splitmix64
    *state = state.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

fn ease_of(s: &str) -> Easing {
    serde_json::from_value(Json::String(s.to_string())).unwrap_or(Easing::Linear)
}

impl ProjectHandle {
    fn with<R>(&self, f: impl FnOnce(&mut Project) -> Result<R, edits_core::EditError>) -> RResult<R> {
        let mut s = self.0.lock();
        f(&mut s.project).map_err(|e| e.to_string().into())
    }
}

/// Build a scripting engine bound to a project handle.
pub fn build_engine(h: &ProjectHandle) -> Engine {
    let mut e = Engine::new();
    e.set_max_operations(50_000_000);
    e.set_max_call_levels(64);
    e.set_max_expr_depths(128, 64);
    e.set_max_array_size(1_000_000);
    e.set_max_map_size(100_000);
    e.set_max_string_size(4_000_000);
    register_common(&mut e);

    e.register_type_with_name::<ProjectHandle>("Project");
    // ---- structure ----
    e.register_fn("add_track", |h: &mut ProjectHandle, name: &str| -> RResult<String> { h.with(|p| ops::add_track(p, None, name, None)) });
    e.register_fn("add_track", |h: &mut ProjectHandle, name: &str, comp: &str| -> RResult<String> { h.with(|p| ops::add_track(p, Some(comp), name, None)) });
    e.register_fn("add_clip", |h: &mut ProjectHandle, track: &str, clip: Map| -> RResult<String> {
        let target = if track.is_empty() { ops::TrackTarget::New } else { ops::TrackTarget::Id(track.to_string()) };
        h.with(|p| ops::add_clip(p, None, target, map_to_json(&clip)))
    });
    e.register_fn("add_clip", |h: &mut ProjectHandle, clip: Map| -> RResult<String> { h.with(|p| ops::add_clip(p, None, ops::TrackTarget::New, map_to_json(&clip))) });
    e.register_fn("add_clip_in", |h: &mut ProjectHandle, comp: &str, track: &str, clip: Map| -> RResult<String> {
        let target = if track.is_empty() { ops::TrackTarget::New } else { ops::TrackTarget::Id(track.to_string()) };
        h.with(|p| ops::add_clip(p, Some(comp), target, map_to_json(&clip)))
    });
    e.register_fn("add_effect", |h: &mut ProjectHandle, target: &str, fx: Map| -> RResult<String> {
        let inst: EffectInstance = serde_json::from_value(map_to_json(&fx)).map_err(|e| e.to_string())?;
        h.with(|p| ops::add_effect(p, target, inst, None))
    });
    e.register_fn("add_comp", |h: &mut ProjectHandle, id: &str, w: i64, hgt: i64, fps: f64, dur: f64| -> RResult<String> {
        h.with(|p| ops::add_comp(p, Some(id), edits_core::Composition::new(id, w as u32, hgt as u32, fps, dur)))
    });
    e.register_fn("update", |h: &mut ProjectHandle, id: &str, patch: Map| -> RResult<()> { h.with(|p| ops::merge_object(p, id, &map_to_json(&patch))) });
    e.register_fn("set", |h: &mut ProjectHandle, id: &str, path: &str, v: Dynamic| -> RResult<()> { h.with(|p| ops::set_path(p, id, path, from_dyn(&v))) });
    e.register_fn("get", |h: &mut ProjectHandle, id: &str, path: &str| -> RResult<Dynamic> { h.with(|p| ops::get_path(p, id, path)).map(|v| to_dyn(&v)) });
    e.register_fn("unset", |h: &mut ProjectHandle, id: &str, path: &str| -> RResult<()> { h.with(|p| ops::unset_path(p, id, path)) });
    e.register_fn("keyframe", |h: &mut ProjectHandle, id: &str, path: &str, t: f64, v: Dynamic, ease: &str| -> RResult<()> {
        let val = to_value(&v).ok_or("unsupported keyframe value")?;
        h.with(|p| ops::add_keyframe(p, id, path, t, val, ease_of(ease)))
    });
    e.register_fn("keyframe", |h: &mut ProjectHandle, id: &str, path: &str, t: f64, v: Dynamic| -> RResult<()> {
        let val = to_value(&v).ok_or("unsupported keyframe value")?;
        h.with(|p| ops::add_keyframe(p, id, path, t, val, Easing::Linear))
    });
    e.register_fn("expr", |h: &mut ProjectHandle, id: &str, path: &str, ex: &str| -> RResult<()> {
        h.with(|p| ops::set_expression(p, id, path, if ex.is_empty() { None } else { Some(ex.to_string()) }))
    });
    e.register_fn("remove", |h: &mut ProjectHandle, id: &str| -> RResult<String> { h.with(|p| ops::remove(p, id)) });
    e.register_fn("split", |h: &mut ProjectHandle, id: &str, t: f64| -> RResult<String> { h.with(|p| ops::split_clip(p, id, t)) });
    e.register_fn("move_clip", |h: &mut ProjectHandle, id: &str, start: f64| -> RResult<()> { h.with(|p| ops::move_clip(p, id, Some(start), None)) });
    e.register_fn("move_clip", |h: &mut ProjectHandle, id: &str, start: f64, track: &str| -> RResult<()> {
        h.with(|p| ops::move_clip(p, id, Some(start), Some(track)))
    });
    e.register_fn("duplicate", |h: &mut ProjectHandle, id: &str, start: f64| -> RResult<String> { h.with(|p| ops::duplicate_clip(p, id, Some(start))) });
    e.register_fn("add_marker", |h: &mut ProjectHandle, t: f64, label: &str| -> RResult<String> {
        h.with(|p| ops::add_marker(p, None, edits_core::Marker { id: String::new(), t, label: label.into(), color: None, duration: 0.0 }))
    });
    e.register_fn("set_duration", |h: &mut ProjectHandle, d: f64| -> RResult<()> {
        h.with(|p| {
            let r = p.root.clone();
            p.compositions.get_mut(&r).map(|c| c.duration = d.max(0.04)).ok_or(edits_core::EditError::NotFound(r))
        })
    });
    e.register_fn("set_var", |h: &mut ProjectHandle, name: &str, v: Dynamic| -> RResult<()> {
        let val = to_value(&v).ok_or("unsupported variable value")?;
        h.with(|p| {
            p.variables.insert(name.to_string(), val);
            Ok(())
        })
    });
    // ---- queries ----
    e.register_fn("clip", |h: &mut ProjectHandle, id: &str| -> RResult<Dynamic> { h.with(|p| ops::get_object(p, id)).map(|v| to_dyn(&v)) });
    e.register_fn("object", |h: &mut ProjectHandle, id: &str| -> RResult<Dynamic> { h.with(|p| ops::get_object(p, id)).map(|v| to_dyn(&v)) });
    e.register_fn("tracks", |h: &mut ProjectHandle| -> Array {
        let s = h.0.lock();
        s.project.root_comp().map(|c| c.tracks.iter().map(|t| Dynamic::from(t.id.clone())).collect()).unwrap_or_default()
    });
    e.register_fn("clips", |h: &mut ProjectHandle, track: &str| -> Array {
        let s = h.0.lock();
        s.project.track(track).map(|t| t.clips.iter().map(|c| Dynamic::from(c.id.clone())).collect()).unwrap_or_default()
    });
    e.register_fn("clips", |h: &mut ProjectHandle| -> Array {
        let s = h.0.lock();
        let root = s.project.root.clone();
        s.project.all_clips().filter(|(c, _, _)| *c == root).map(|(_, _, c)| Dynamic::from(c.id.clone())).collect()
    });
    e.register_fn("assets", |h: &mut ProjectHandle| -> Array { h.0.lock().project.assets.keys().map(|k| Dynamic::from(k.clone())).collect() });
    e.register_fn("assets", |h: &mut ProjectHandle, kind: &str| -> Array {
        let s = h.0.lock();
        s.project
            .assets
            .iter()
            .filter(|(_, a)| serde_json::to_value(a.kind).ok().and_then(|v| v.as_str().map(|x| x == kind)).unwrap_or(false))
            .map(|(k, _)| Dynamic::from(k.clone()))
            .collect()
    });
    e.register_fn("asset", |h: &mut ProjectHandle, id: &str| -> RResult<Dynamic> { h.with(|p| ops::get_object(p, id)).map(|v| to_dyn(&v)) });
    e.register_fn("asset_duration", |h: &mut ProjectHandle, id: &str| -> f64 {
        let s = h.0.lock();
        s.project.assets.get(id).and_then(|a| crate::media_pool::MediaPool::duration(a)).unwrap_or(0.0)
    });
    e.register_fn("comp", |h: &mut ProjectHandle| -> Dynamic {
        let s = h.0.lock();
        s.project
            .root_comp()
            .map(|c| to_dyn(&json!({"id": s.project.root, "width": c.width, "height": c.height, "fps": c.fps, "duration": c.duration})))
            .unwrap_or(Dynamic::UNIT)
    });
    e.register_fn("json", |h: &mut ProjectHandle| -> Dynamic { to_dyn(&serde_json::to_value(&h.0.lock().project).unwrap_or_default()) });
    e.register_fn("new_id", |h: &mut ProjectHandle, prefix: &str| -> String { h.0.lock().project.new_id(prefix) });
    // ---- timing ----
    let timing_list = |h: &ProjectHandle, which: &str| -> Array {
        let s = h.0.lock();
        let t = &s.project.timing;
        let list = match which {
            "beats" => &t.beats,
            "downbeats" => &t.downbeats,
            "drops" => &t.drops,
            _ => &t.accents,
        };
        list.iter().map(|b| Dynamic::from_float(b + t.offset)).collect()
    };
    for which in ["beats", "downbeats", "drops", "accents"] {
        let hh = h.clone();
        e.register_fn(which, move || timing_list(&hh, which));
    }
    let hh = h.clone();
    e.register_fn("bpm", move || hh.0.lock().project.timing.bpm.unwrap_or(0.0));
    let hh = h.clone();
    e.register_fn("sections", move || -> Array {
        let s = hh.0.lock();
        let off = s.project.timing.offset;
        s.project
            .timing
            .sections
            .iter()
            .map(|x| to_dyn(&json!({"start": x.start + off, "end": x.end + off, "label": x.label, "energy": x.energy})))
            .collect()
    });
    let hh = h.clone();
    e.register_fn("beats_between", move |a: f64, b: f64| -> Array {
        timing_list(&hh, "beats").into_iter().filter(|x| {
            let v = f(x);
            v >= a && v < b
        })
        .collect()
    });
    let hh = h.clone();
    e.register_fn("nearest_beat", move |t: f64| -> f64 {
        let s = hh.0.lock();
        ops::snap_to_beat(&s.project, t, f64::MAX)
    });
    let hh = h.clone();
    e.register_fn("scenes", move |asset: &str| -> RResult<Dynamic> {
        let (proj, svc) = {
            let s = hh.0.lock();
            (s.project.clone(), s.services.clone())
        };
        svc.scenes(&proj, asset).map(|v| to_dyn(&v)).map_err(|e| e.into())
    });
    let hh = h.clone();
    e.register_fn("subtitles", move |asset: &str| -> RResult<Dynamic> {
        let (proj, svc) = {
            let s = hh.0.lock();
            (s.project.clone(), s.services.clone())
        };
        svc.subtitles(&proj, asset).map(|v| to_dyn(&v)).map_err(|e| e.into())
    });
    // ---- randomness (deterministic per run) ----
    let hh = h.clone();
    e.register_fn("rand", move || next_rand(&mut hh.0.lock().rng));
    let hh = h.clone();
    e.register_fn("rand", move |a: f64, b: f64| a + (b - a) * next_rand(&mut hh.0.lock().rng));
    let hh = h.clone();
    e.register_fn("rand_int", move |a: i64, b: i64| -> i64 { a + (next_rand(&mut hh.0.lock().rng) * ((b - a + 1).max(1)) as f64).floor() as i64 });
    let hh = h.clone();
    e.register_fn("choose", move |arr: Array| -> Dynamic {
        if arr.is_empty() {
            return Dynamic::UNIT;
        }
        let i = (next_rand(&mut hh.0.lock().rng) * arr.len() as f64).floor() as usize;
        arr[i.min(arr.len() - 1)].clone()
    });
    let hh = h.clone();
    e.register_fn("shuffle", move |mut arr: Array| -> Array {
        let mut s = hh.0.lock();
        for i in (1..arr.len()).rev() {
            let j = (next_rand(&mut s.rng) * (i + 1) as f64).floor() as usize;
            arr.swap(i, j.min(i));
        }
        arr
    });
    let hh = h.clone();
    e.register_fn("log", move |msg: Dynamic| {
        let mut s = hh.0.lock();
        if s.logs.len() < 500 {
            s.logs.push(msg.to_string());
        }
    });
    let hh = h.clone();
    e.on_print(move |msg| {
        let mut s = hh.0.lock();
        if s.logs.len() < 500 {
            s.logs.push(msg.to_string());
        }
    });
    // ---- nested presets ----
    e.register_fn("apply_preset", |h: &mut ProjectHandle, target: &str, preset: &str, args: Map| -> RResult<()> {
        let (proj, lib, svc, depth, seed) = {
            let s = h.0.lock();
            (s.project.clone(), s.lib.clone(), s.services.clone(), s.depth, s.rng)
        };
        if depth > 8 {
            return err("preset nesting too deep");
        }
        let def = lib.preset(preset).cloned().ok_or_else(|| format!("unknown preset '{preset}'"))?;
        let a = match map_to_json(&args) {
            Json::Object(m) => m,
            _ => Default::default(),
        };
        let out = run_preset(proj, lib, svc, &def, if target.is_empty() { None } else { Some(target) }, &a, None, depth + 1, seed)
            .map_err(|e| e.to_string())?;
        let mut s = h.0.lock();
        s.project = out.project;
        s.logs.extend(out.logs);
        Ok(())
    });

    // ---- map helpers ----
    e.register_fn("fx", |id: &str| -> Map {
        let mut m = Map::new();
        m.insert("effect".into(), Dynamic::from(id.to_string()));
        m
    });
    e.register_fn("fx", |id: &str, params: Map| -> Map {
        let mut m = Map::new();
        m.insert("effect".into(), Dynamic::from(id.to_string()));
        m.insert("params".into(), Dynamic::from_map(params));
        m
    });
    e.register_fn("keys", |arr: Array| -> Map {
        let mut m = Map::new();
        m.insert("keyframes".into(), Dynamic::from_array(arr));
        m
    });
    e.register_fn("expr", |s: &str| -> Map {
        let mut m = Map::new();
        m.insert("expr".into(), Dynamic::from(s.to_string()));
        m
    });
    e.register_fn("expr", |s: &str, base: Dynamic| -> Map {
        let mut m = Map::new();
        m.insert("expr".into(), Dynamic::from(s.to_string()));
        m.insert("value".into(), base);
        m
    });
    e.register_fn("pulse_keys", |times: Array, base: Dynamic, peak: Dynamic, attack: f64, release: f64, ease: &str| -> Map {
        pulse_keys(&times, &base, &peak, attack, release, ease)
    });
    e.register_fn("pulse_keys", |times: Array, base: Dynamic, peak: Dynamic, release: f64| -> Map { pulse_keys(&times, &base, &peak, 0.0, release, "ease_out_cubic") });
    e.register_fn("add_fx", |clip: &mut Map, fx: Map| {
        let list = clip.entry("effects".into()).or_insert_with(|| Dynamic::from_array(vec![]));
        if let Some(mut arr) = list.write_lock::<Array>() {
            arr.push(Dynamic::from_map(fx));
        } else {
            *list = Dynamic::from_array(vec![Dynamic::from_map(fx)]);
        }
    });
    e.register_fn("set_kf", |clip: &mut Map, path: &str, t: f64, v: Dynamic, ease: &str| -> RResult<()> {
        json_edit(clip, |j| {
            let slot = ops::path_get_mut(j, path).ok_or("bad path")?;
            let mut prop: edits_core::Property<Value> = if slot.is_null() {
                edits_core::Property::Static(to_value(&v).ok_or("bad value")?)
            } else {
                serde_json::from_value(slot.clone()).map_err(|e| e.to_string())?
            };
            prop.set_keyframe(t, to_value(&v).ok_or("bad value")?, ease_of(ease));
            *slot = serde_json::to_value(&prop).map_err(|e| e.to_string())?;
            Ok(())
        })
    });
    e.register_fn("animate", |clip: &mut Map, path: &str, keys: Array| -> RResult<()> {
        json_edit(clip, |j| {
            let slot = ops::path_get_mut(j, path).ok_or("bad path")?;
            let ex = slot.get("expr").cloned();
            let mut m = serde_json::Map::new();
            m.insert("keyframes".into(), from_dyn(&Dynamic::from_array(keys.clone())));
            if let Some(x) = ex {
                m.insert("expr".into(), x);
            }
            *slot = Json::Object(m);
            Ok(())
        })
    });
    e.register_fn("get_path", |m: &mut Map, path: &str| -> Dynamic {
        let j = map_to_json(m);
        ops::path_get(&j, path).map(to_dyn).unwrap_or(Dynamic::UNIT)
    });
    e.register_fn("set_path", |m: &mut Map, path: &str, v: Dynamic| -> RResult<()> {
        json_edit(m, |j| {
            *ops::path_get_mut(j, path).ok_or("bad path")? = from_dyn(&v);
            Ok(())
        })
    });
    e.register_fn("merge", |m: &mut Map, patch: Map| {
        let mut j = map_to_json(m);
        json_patch::merge(&mut j, &map_to_json(&patch));
        if let Some(nm) = to_dyn(&j).try_cast::<Map>() {
            *m = nm;
        }
    });
    e.register_fn("ease_value", |name: &str, x: f64| ease_of(name).apply(x, 1.0));
    // keyframes for a single hit: base before `at`, jump to `peak` at `at`, settle to base after `dur`
    e.register_fn("hit_keys", |at: f64, base: Dynamic, peak: Dynamic, dur: f64, ease: &str| -> Map { hit_keys(at, &base, &peak, dur, ease) });
    e.register_fn("hit_keys", |at: f64, base: Dynamic, peak: Dynamic, dur: f64| -> Map { hit_keys(at, &base, &peak, dur, "ease_out_expo") });
    // times from ctx: on = beats | downbeats | drops | accents | half (every other beat) | bars2 (every 2 bars)
    e.register_fn("pick_times", |ctx: Map, on: &str, every: f64| -> Array { pick_times(&ctx, on, every.round() as i64) });
    e.register_fn("pick_times", |ctx: Map, on: &str, every: i64| -> Array { pick_times(&ctx, on, every) });
    #[allow(unused)]
    let _old = |ctx: Map, on: &str, every: i64| -> Array {
        let get = |k: &str| ctx.get(k).and_then(|v| v.clone().try_cast::<Array>()).unwrap_or_default();
        let list = match on {
            "downbeats" | "bars" => get("downbeats"),
            "drops" => get("drops"),
            "accents" => get("accents"),
            "half" => get("beats").into_iter().step_by(2).collect(),
            "bars2" => get("downbeats").into_iter().step_by(2).collect(),
            _ => get("beats"),
        };
        list.into_iter().step_by(every.max(1) as usize).collect()
    };
    e.register_fn("every", |arr: Array, n: i64| -> Array { arr.into_iter().step_by(n.max(1) as usize).collect() });
    e.register_fn("every", |arr: Array, n: f64| -> Array { arr.into_iter().step_by((n.round() as usize).max(1)).collect() });
    e.register_fn("dir_vec", |name: &str, dist: f64| -> Array {
        let (x, y) = match name {
            "left" => (-dist, 0.0),
            "right" => (dist, 0.0),
            "up" | "top" => (0.0, -dist),
            "down" | "bottom" => (0.0, dist),
            "up_left" => (-dist * 0.707, -dist * 0.707),
            "up_right" => (dist * 0.707, -dist * 0.707),
            "down_left" => (-dist * 0.707, dist * 0.707),
            "down_right" => (dist * 0.707, dist * 0.707),
            _ => (-dist, 0.0),
        };
        vec![Dynamic::from_float(x), Dynamic::from_float(y)]
    });
    e.register_fn("has", |m: &mut Map, k: &str| m.contains_key(k));
    e.register_fn("to_json", |d: Dynamic| -> String { serde_json::to_string(&from_dyn(&d)).unwrap_or_default() });
    e.register_fn("parse_json", |s: &str| -> Dynamic { serde_json::from_str::<Json>(s).map(|v| to_dyn(&v)).unwrap_or(Dynamic::UNIT) });
    e
}

fn json_edit(m: &mut Map, f: impl FnOnce(&mut Json) -> Result<(), String>) -> RResult<()> {
    let mut j = map_to_json(m);
    f(&mut j).map_err(|e| -> Box<EvalAltResult> { e.into() })?;
    match to_dyn(&j).try_cast::<Map>() {
        Some(nm) => {
            *m = nm;
            Ok(())
        }
        None => err("edit produced a non-map"),
    }
}

fn pick_times(ctx: &Map, on: &str, every: i64) -> Array {
    let get = |k: &str| ctx.get(k).and_then(|v| v.clone().try_cast::<Array>()).unwrap_or_default();
    let list = match on {
        "downbeats" | "bars" => get("downbeats"),
        "drops" => get("drops"),
        "accents" => get("accents"),
        "half" => get("beats").into_iter().step_by(2).collect(),
        "bars2" => get("downbeats").into_iter().step_by(2).collect(),
        _ => get("beats"),
    };
    list.into_iter().step_by(every.max(1) as usize).collect()
}

fn hit_keys(at: f64, base: &Dynamic, peak: &Dynamic, dur: f64, ease: &str) -> Map {
    let mk = |t: f64, v: &Dynamic, e: &str| -> Dynamic { Dynamic::from_array(vec![Dynamic::from_float(t), v.clone(), Dynamic::from(e.to_string())]) };
    let mut keys = vec![];
    if at > 0.0 {
        keys.push(mk((at - 1e-3).max(0.0), base, "hold"));
    }
    keys.push(mk(at.max(0.0), peak, ease));
    keys.push(mk(at.max(0.0) + dur.max(1e-3), base, "linear"));
    let mut m = Map::new();
    m.insert("keyframes".into(), Dynamic::from_array(keys));
    m
}

fn pulse_keys(times: &Array, base: &Dynamic, peak: &Dynamic, attack: f64, release: f64, ease: &str) -> Map {
    let mut ts: Vec<f64> = times.iter().map(f).collect();
    ts.sort_by(|a, b| a.total_cmp(b));
    let mut keys: Vec<Dynamic> = vec![];
    let mk = |t: f64, v: &Dynamic, e: &str| -> Dynamic { Dynamic::from_array(vec![Dynamic::from_float(t), v.clone(), Dynamic::from(e.to_string())]) };
    if let Some(first) = ts.first() {
        if *first - attack > 0.0 {
            keys.push(mk((first - attack - 1e-3).max(0.0), base, "hold"));
        }
    }
    for (i, t) in ts.iter().enumerate() {
        let next = ts.get(i + 1).copied().unwrap_or(f64::MAX);
        if attack > 0.0 {
            keys.push(mk((t - attack).max(0.0), base, "ease_in_quad"));
        }
        keys.push(mk(*t, peak, ease));
        let end = (t + release).min(next - attack - 1e-3).max(*t + 1e-3);
        keys.push(mk(end, base, "hold"));
    }
    let mut m = Map::new();
    m.insert("keyframes".into(), Dynamic::from_array(keys));
    m
}

fn ctx_map(project: &Project, clip: Option<&edits_core::Clip>, at: Option<f64>) -> Map {
    let comp = clip.and_then(|c| project.comp_of(&c.id)).or(project.root_comp());
    let mut m = Map::new();
    if let Some(c) = comp {
        m.insert("fps".into(), Dynamic::from_float(c.fps));
        m.insert("width".into(), Dynamic::from_float(c.width as f64));
        m.insert("height".into(), Dynamic::from_float(c.height as f64));
        m.insert("comp_duration".into(), Dynamic::from_float(c.duration));
    }
    let t = &project.timing;
    let bpm = t.bpm.unwrap_or(0.0);
    m.insert("bpm".into(), Dynamic::from_float(bpm));
    m.insert("beat_len".into(), Dynamic::from_float(if bpm > 0.0 { 60.0 / bpm } else { 0.5 }));
    let rel = |list: &[f64], start: f64, end: f64| -> Array {
        list.iter().map(|b| b + t.offset).filter(|b| *b >= start - 1e-6 && *b < end).map(|b| Dynamic::from_float(b - start)).collect()
    };
    match clip {
        Some(c) => {
            m.insert("clip_id".into(), Dynamic::from(c.id.clone()));
            m.insert("clip_start".into(), Dynamic::from_float(c.start));
            m.insert("clip_end".into(), Dynamic::from_float(c.end()));
            m.insert("clip_duration".into(), Dynamic::from_float(c.duration));
            m.insert("beats".into(), Dynamic::from_array(rel(&t.beats, c.start, c.end())));
            m.insert("downbeats".into(), Dynamic::from_array(rel(&t.downbeats, c.start, c.end())));
            m.insert("drops".into(), Dynamic::from_array(rel(&t.drops, c.start, c.end())));
            m.insert("accents".into(), Dynamic::from_array(rel(&t.accents, c.start, c.end())));
            m.insert("kind".into(), Dynamic::from(c.source.kind_name().to_string()));
        }
        None => {
            m.insert("beats".into(), Dynamic::from_array(rel(&t.beats, 0.0, f64::MAX)));
            m.insert("downbeats".into(), Dynamic::from_array(rel(&t.downbeats, 0.0, f64::MAX)));
            m.insert("drops".into(), Dynamic::from_array(rel(&t.drops, 0.0, f64::MAX)));
            m.insert("accents".into(), Dynamic::from_array(rel(&t.accents, 0.0, f64::MAX)));
        }
    }
    m.insert("at".into(), at.map(Dynamic::from_float).unwrap_or(Dynamic::UNIT));
    m
}

/// Run a preset. Clip presets need `target` (clip id); timeline presets ignore it.
#[allow(clippy::too_many_arguments)]
pub fn run_preset(
    project: Project,
    lib: Arc<Library>,
    services: Arc<dyn ScriptServices>,
    def: &PresetDef,
    target: Option<&str>,
    args: &serde_json::Map<String, Json>,
    at: Option<f64>,
    depth: u32,
    seed: u64,
) -> anyhow::Result<ScriptOutcome> {
    let resolved = def.resolve_args(args)?;
    let seed = resolved.get("seed").and_then(|v| v.as_u64()).unwrap_or_else(|| {
        crate::media_pool::key_of((seed, &def.id, target.unwrap_or(""), serde_json::to_string(&resolved).unwrap_or_default()))
    });
    let clip = match (def.scope, target) {
        (PresetScope::Clip, Some(id)) => Some(project.clip(id).cloned().ok_or_else(|| anyhow::anyhow!("clip '{id}' not found"))?),
        (PresetScope::Clip, None) => anyhow::bail!("preset '{}' applies to a clip: pass a clip id as target", def.id),
        (PresetScope::Timeline, _) => None,
    };
    let ctx = ctx_map(&project, clip.as_ref(), at);
    let handle = ProjectHandle(Arc::new(Mutex::new(ScriptState { project, lib, logs: vec![], services, depth, rng: seed })));
    let engine = build_engine(&handle);
    let mut scope = Scope::new();
    scope.push("project", handle.clone());
    scope.push("args", to_dyn(&Json::Object(resolved)));
    scope.push("ctx", Dynamic::from_map(ctx));
    if let Some(c) = &clip {
        scope.push("clip", to_dyn(&serde_json::to_value(c)?));
    }
    let result = engine
        .eval_with_scope::<Dynamic>(&mut scope, def.script())
        .map_err(|e| anyhow::anyhow!("preset '{}' failed: {e}", def.id))?;
    let clip_out = scope.get_value::<Dynamic>("clip");
    drop(scope);
    drop(engine);
    let state = Arc::try_unwrap(handle.0).map_err(|_| anyhow::anyhow!("script kept a project reference"))?.into_inner();
    let mut project = state.project;
    if let (Some(c), Some(d)) = (&clip, clip_out) {
        let mut j = from_dyn(&d);
        let original = serde_json::to_value(c)?;
        if j == original {
            ops::assign_missing_ids(&mut project);
            return Ok(ScriptOutcome { project, logs: state.logs, result: from_dyn(&result) });
        }
        if let Some(o) = j.as_object_mut() {
            o.insert("id".into(), Json::String(c.id.clone()));
        }
        if project.find_clip(&c.id).is_some() {
            ops::set_object(&mut project, &c.id, j).map_err(|e| anyhow::anyhow!("preset '{}' produced an invalid clip: {e}", def.id))?;
        }
    }
    ops::assign_missing_ids(&mut project);
    Ok(ScriptOutcome { project, logs: state.logs, result: from_dyn(&result) })
}

/// Run arbitrary agent code with the timeline environment.
pub fn run_script(project: Project, lib: Arc<Library>, services: Arc<dyn ScriptServices>, code: &str, args: Json) -> anyhow::Result<ScriptOutcome> {
    let ctx = ctx_map(&project, None, None);
    let handle = ProjectHandle(Arc::new(Mutex::new(ScriptState { project, lib, logs: vec![], services, depth: 0, rng: 0x5EED })));
    let engine = build_engine(&handle);
    let mut scope = Scope::new();
    scope.push("project", handle.clone());
    scope.push("args", to_dyn(&args));
    scope.push("ctx", Dynamic::from_map(ctx));
    let result = engine.eval_with_scope::<Dynamic>(&mut scope, code).map_err(|e| anyhow::anyhow!("script failed: {e}"))?;
    drop(scope);
    drop(engine);
    let state = Arc::try_unwrap(handle.0).map_err(|_| anyhow::anyhow!("script kept a project reference"))?.into_inner();
    let mut project = state.project;
    ops::assign_missing_ids(&mut project);
    Ok(ScriptOutcome { project, logs: state.logs, result: from_dyn(&result) })
}

/// Check that a preset script compiles.
pub fn check_script(code: &str) -> Result<(), String> {
    let h = ProjectHandle(Arc::new(Mutex::new(ScriptState {
        project: Project::new("x", 16, 16, 24.0, 1.0),
        lib: Arc::new(Library::default()),
        logs: vec![],
        services: Arc::new(NoServices),
        depth: 0,
        rng: 0,
    })));
    build_engine(&h).compile(code).map(|_| ()).map_err(|e| e.to_string())
}

#[allow(dead_code)]
fn unused(_: ImmutableString, _: Value) -> Dynamic {
    from_value(&Value::Num(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_script_builds_clips() {
        let mut p = Project::new("t", 640, 360, 24.0, 8.0);
        p.timing.beats = vec![0.0, 0.5, 1.0, 1.5, 2.0];
        let code = r#"
            let tr = project.add_track("cuts");
            let bs = beats();
            for i in 0..bs.len() - 1 {
                let id = project.add_clip(tr, #{ start: bs[i], duration: bs[i+1] - bs[i],
                    source: #{ type: "solid", color: hsv(i * 60.0, 1.0, 1.0) } });
                project.add_effect(id, fx("glow", #{ intensity: 2.0 }));
            }
            log("made " + (bs.len() - 1) + " clips");
            bs.len() - 1
        "#;
        let out = run_script(p, Arc::new(Library::builtin().clone()), Arc::new(NoServices), code, Json::Null).unwrap();
        assert_eq!(out.result, json!(4));
        assert_eq!(out.project.root_comp().unwrap().tracks[0].clips.len(), 4);
        assert_eq!(out.logs, vec!["made 4 clips"]);
    }

    #[test]
    fn clip_preset_edits_clip() {
        let mut p = Project::new("t", 640, 360, 24.0, 8.0);
        let id = ops::add_clip(&mut p, None, ops::TrackTarget::New, json!({"source": {"type": "solid", "color": "red"}, "duration": 2})).unwrap();
        let src = r#"//! id = "test_punch"
//! scope = "clip"
//! params = [ { name = "strength", type = "float", default = 0.3 } ]
clip.transform = #{ scale: keys([[0.0, 1.0 + args.strength], [0.3, 1.0, "punch"]]) };
clip.add_fx(fx("rgb_split", #{ amount: pulse_keys([0.0], 0.0, 20.0, 0.25) }));
"#;
        let def = PresetDef::parse(src, "x", false).unwrap();
        let out = run_preset(p, Arc::new(Library::builtin().clone()), Arc::new(NoServices), &def, Some(&id), &Default::default(), None, 0, 1).unwrap();
        let c = out.project.clip(&id).unwrap();
        assert!(c.transform.scale.is_animated());
        assert_eq!(c.effects.len(), 1);
        assert!(!c.effects[0].id.is_empty());
    }
}
