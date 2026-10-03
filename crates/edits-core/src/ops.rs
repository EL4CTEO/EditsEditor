//! Edit operations. These are the primitives used by the MCP server, scripts, presets and the CLI.
//! Every operation validates its result by round-tripping through the typed model, so an agent
//! can never leave the project in an unparseable state.

use serde_json::{Map, Value as Json};

use crate::{
    EditError, Result,
    easing::Easing,
    model::{Clip, ClipSource, Composition, EffectInstance, Marker, Project, Track},
    property::Property,
    query::{EffectOwner, ObjectRef},
    value::Value,
};

// ------------------------------------------------------------------------------------------------
// Generic object access
// ------------------------------------------------------------------------------------------------

/// Serialize the object with `id` to JSON.
pub fn get_object(p: &Project, id: &str) -> Result<Json> {
    let r = p.find(id).ok_or_else(|| EditError::NotFound(id.to_string()))?;
    let v = match &r {
        ObjectRef::Comp(c) => serde_json::to_value(&p.compositions[c]),
        ObjectRef::Asset(a) => serde_json::to_value(&p.assets[a]),
        ObjectRef::Track(t) => serde_json::to_value(&p.compositions[&t.comp].tracks[t.track]),
        ObjectRef::Clip(c) => serde_json::to_value(p.clip_at(c).unwrap()),
        ObjectRef::Effect(e) => serde_json::to_value(&p.effect_list(&e.owner).unwrap()[e.index]),
        ObjectRef::Marker { comp, index } => serde_json::to_value(&p.compositions[comp].markers[*index]),
    }?;
    Ok(v)
}

/// Replace the object with `id` by a JSON value (validated against the model).
pub fn set_object(p: &mut Project, id: &str, mut v: Json) -> Result<()> {
    let r = p.find(id).ok_or_else(|| EditError::NotFound(id.to_string()))?;
    // Keep the id stable unless the caller explicitly changes it.
    if let Some(o) = v.as_object_mut() {
        if !matches!(r, ObjectRef::Comp(_) | ObjectRef::Asset(_)) && !o.contains_key("id") {
            o.insert("id".into(), Json::String(id.to_string()));
        }
    }
    match r {
        ObjectRef::Comp(c) => {
            let comp: Composition = from_json(v, "composition")?;
            p.compositions[&c] = comp;
        }
        ObjectRef::Asset(a) => {
            p.assets[&a] = from_json(v, "asset")?;
        }
        ObjectRef::Track(t) => {
            let track: Track = from_json(v, "track")?;
            p.compositions[&t.comp].tracks[t.track] = track;
        }
        ObjectRef::Clip(c) => {
            let clip: Clip = from_json(v, "clip")?;
            *p.clip_at_mut(&c).unwrap() = clip;
            sort_track(p, &c.comp, c.track);
        }
        ObjectRef::Effect(e) => {
            let fx: EffectInstance = from_json(v, "effect")?;
            p.effect_list_mut(&e.owner).unwrap()[e.index] = fx;
        }
        ObjectRef::Marker { comp, index } => {
            p.compositions[&comp].markers[index] = from_json(v, "marker")?;
        }
    }
    assign_missing_ids(p);
    Ok(())
}

/// RFC 7396 JSON merge patch on an object. `null` values delete fields (reset to default).
pub fn merge_object(p: &mut Project, id: &str, patch: &Json) -> Result<()> {
    let mut v = get_object(p, id)?;
    json_patch::merge(&mut v, patch);
    set_object(p, id, v)
}

fn from_json<T: serde::de::DeserializeOwned>(v: Json, what: &str) -> Result<T> {
    serde_json::from_value(v).map_err(|e| EditError::Invalid(format!("invalid {what}: {e}")))
}

/// Resolve a dot path (`transform.scale`, `effects.fx3.params.amount`, `tracks.0.clips.c2`) inside a JSON value.
/// Array segments may be numeric indices or element ids.
pub fn path_get<'a>(root: &'a Json, path: &str) -> Option<&'a Json> {
    if path.is_empty() {
        return Some(root);
    }
    if path.starts_with('/') {
        return root.pointer(path);
    }
    let mut cur = root;
    for seg in path.split('.') {
        cur = match cur {
            Json::Object(m) => m.get(seg)?,
            Json::Array(a) => match seg.parse::<usize>() {
                Ok(i) => a.get(i)?,
                Err(_) => a.iter().find(|x| x.get("id").and_then(|i| i.as_str()) == Some(seg))?,
            },
            _ => return None,
        };
    }
    Some(cur)
}

/// Mutable path resolution that creates missing object fields.
pub fn path_get_mut<'a>(root: &'a mut Json, path: &str) -> Option<&'a mut Json> {
    if path.is_empty() {
        return Some(root);
    }
    if path.starts_with('/') {
        return root.pointer_mut(path);
    }
    let mut cur = root;
    for seg in path.split('.') {
        cur = match cur {
            Json::Object(m) => m.entry(seg.to_string()).or_insert(Json::Null),
            Json::Array(a) => match seg.parse::<usize>() {
                Ok(i) => a.get_mut(i)?,
                Err(_) => a.iter_mut().find(|x| x.get("id").and_then(|i| i.as_str()) == Some(seg))?,
            },
            Json::Null => {
                *cur = Json::Object(Map::new());
                match cur {
                    Json::Object(m) => m.entry(seg.to_string()).or_insert(Json::Null),
                    _ => unreachable!(),
                }
            }
            _ => return None,
        };
    }
    Some(cur)
}

/// Read a property/field of an object by path.
pub fn get_path(p: &Project, id: &str, path: &str) -> Result<Json> {
    let v = get_object(p, id)?;
    path_get(&v, path).cloned().ok_or_else(|| EditError::NotFound(format!("{id}.{path}")))
}

/// Set a property/field of an object by path (creates intermediate objects).
pub fn set_path(p: &mut Project, id: &str, path: &str, value: Json) -> Result<()> {
    let mut v = get_object(p, id)?;
    let slot = path_get_mut(&mut v, path).ok_or_else(|| EditError::NotFound(format!("{id}.{path}")))?;
    *slot = value;
    set_object(p, id, v)
}

/// Remove a field by path (resets it to its default).
pub fn unset_path(p: &mut Project, id: &str, path: &str) -> Result<()> {
    let mut v = get_object(p, id)?;
    let (parent, last) = match path.rsplit_once('.') {
        Some((a, b)) => (a, b),
        None => ("", path),
    };
    let par = path_get_mut(&mut v, parent).ok_or_else(|| EditError::NotFound(format!("{id}.{path}")))?;
    match par {
        Json::Object(m) => {
            m.shift_remove(last);
        }
        Json::Array(a) => {
            if let Ok(i) = last.parse::<usize>() {
                if i < a.len() {
                    a.remove(i);
                }
            } else {
                a.retain(|x| x.get("id").and_then(|i| i.as_str()) != Some(last));
            }
        }
        _ => {}
    }
    set_object(p, id, v)
}

/// Add/replace a keyframe on an animatable property addressed by path.
pub fn add_keyframe(p: &mut Project, id: &str, path: &str, t: f64, v: Value, ease: Easing) -> Result<()> {
    let mut obj = get_object(p, id)?;
    let slot = path_get_mut(&mut obj, path).ok_or_else(|| EditError::NotFound(format!("{id}.{path}")))?;
    let mut prop: Property<Value> = if slot.is_null() {
        Property::Static(v.clone())
    } else {
        serde_json::from_value(slot.clone()).map_err(|e| EditError::Invalid(format!("{path} is not animatable: {e}")))?
    };
    prop.set_keyframe(t, v, ease);
    *slot = serde_json::to_value(&prop)?;
    set_object(p, id, obj)
}

/// Replace all keyframes on a property (keeps an existing expression).
pub fn set_keyframes(p: &mut Project, id: &str, path: &str, keyframes: Json) -> Result<()> {
    let mut obj = get_object(p, id)?;
    let slot = path_get_mut(&mut obj, path).ok_or_else(|| EditError::NotFound(format!("{id}.{path}")))?;
    let expr = slot.get("expr").cloned();
    let mut m = Map::new();
    m.insert("keyframes".into(), keyframes);
    if let Some(e) = expr {
        m.insert("expr".into(), e);
    }
    *slot = Json::Object(m);
    set_object(p, id, obj)
}

/// Attach (or clear with `None`) an expression on a property.
pub fn set_expression(p: &mut Project, id: &str, path: &str, expr: Option<String>) -> Result<()> {
    let mut obj = get_object(p, id)?;
    let slot = path_get_mut(&mut obj, path).ok_or_else(|| EditError::NotFound(format!("{id}.{path}")))?;
    let mut prop: Property<Value> = if slot.is_null() {
        Property::Animated(Box::new(crate::property::Animated {
            keyframes: vec![],
            value: None,
            expr: None,
            r#loop: Default::default(),
        }))
    } else {
        serde_json::from_value(slot.clone()).map_err(|e| EditError::Invalid(format!("{path} is not animatable: {e}")))?
    };
    prop.set_expr(expr);
    *slot = serde_json::to_value(&prop)?;
    set_object(p, id, obj)
}

// ------------------------------------------------------------------------------------------------
// Structural operations
// ------------------------------------------------------------------------------------------------

/// Resolve a composition id (`None` = root).
pub fn comp_id(p: &Project, comp: Option<&str>) -> Result<String> {
    let id = comp.unwrap_or(&p.root).to_string();
    if p.compositions.contains_key(&id) { Ok(id) } else { Err(EditError::NotFound(id)) }
}

pub fn add_comp(p: &mut Project, id: Option<&str>, comp: Composition) -> Result<String> {
    let id = match id {
        Some(i) if !i.is_empty() => {
            if p.id_exists(i) {
                return Err(EditError::Invalid(format!("id '{i}' already exists")));
            }
            i.to_string()
        }
        _ => p.new_id("comp"),
    };
    p.compositions.insert(id.clone(), comp);
    Ok(id)
}

/// Add a track. `index` = position from the bottom (None = top).
pub fn add_track(p: &mut Project, comp: Option<&str>, name: &str, index: Option<usize>) -> Result<String> {
    let cid = comp_id(p, comp)?;
    let id = p.new_id("t");
    let track = Track::new(id.clone(), name);
    let tracks = &mut p.compositions[&cid].tracks;
    let i = index.unwrap_or(tracks.len()).min(tracks.len());
    tracks.insert(i, track);
    Ok(id)
}

/// Where to put a new clip.
#[derive(Clone, Debug, Default)]
pub enum TrackTarget {
    /// Create a new track on top.
    #[default]
    New,
    Id(String),
    Index(usize),
}

/// Add a clip from (partial) JSON. Missing `id`, `start` (= end of last clip on the track) and
/// `duration` (= media duration / speed, or 3 s) are filled in.
pub fn add_clip(p: &mut Project, comp: Option<&str>, target: TrackTarget, mut clip: Json) -> Result<String> {
    let cid = comp_id(p, comp)?;
    let ti = match target {
        TrackTarget::New => {
            let name = clip.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
            let tid = add_track(p, Some(&cid), &name, None)?;
            p.compositions[&cid].tracks.iter().position(|t| t.id == tid).unwrap()
        }
        TrackTarget::Id(t) => p.compositions[&cid]
            .tracks
            .iter()
            .position(|tr| tr.id == t)
            .ok_or_else(|| EditError::NotFound(format!("track '{t}' in comp '{cid}'")))?,
        TrackTarget::Index(i) => {
            let n = p.compositions[&cid].tracks.len();
            if i >= n {
                for _ in n..=i {
                    add_track(p, Some(&cid), "", None)?;
                }
            }
            i
        }
    };
    let obj = clip.as_object_mut().ok_or_else(|| EditError::Invalid("clip must be a JSON object".into()))?;

    let id = match obj.get("id").and_then(|v| v.as_str()) {
        Some(i) if !i.is_empty() => {
            if p.id_exists(i) {
                return Err(EditError::Invalid(format!("id '{i}' already exists")));
            }
            i.to_string()
        }
        _ => p.new_id("c"),
    };
    obj.insert("id".into(), Json::String(id.clone()));

    if !obj.contains_key("start") {
        let end = p.compositions[&cid].tracks[ti].clips.iter().map(|c| c.end()).fold(0.0, f64::max);
        obj.insert("start".into(), end.into());
    }
    if !obj.contains_key("duration") {
        let src: Option<ClipSource> = obj.get("source").and_then(|s| serde_json::from_value(s.clone()).ok());
        let speed = obj.get("speed").and_then(|s| s.as_f64()).unwrap_or(1.0).abs().max(1e-6);
        let src_in = obj.get("in").and_then(|s| s.as_f64()).unwrap_or(0.0);
        let dur = match src {
            Some(ClipSource::Media { asset, .. }) => p
                .assets
                .get(&asset)
                .and_then(|a| a.info.as_ref())
                .and_then(|i| i.duration)
                .map(|d| ((d - src_in).max(0.04)) / speed)
                .unwrap_or(3.0),
            Some(ClipSource::Comp { comp }) => p.compositions.get(&comp).map(|c| c.duration).unwrap_or(3.0),
            _ => 3.0,
        };
        obj.insert("duration".into(), dur.into());
    }
    let clip: Clip = from_json(clip, "clip")?;
    if let ClipSource::Media { asset, .. } = &clip.source {
        if !p.assets.contains_key(asset) {
            return Err(EditError::NotFound(format!("asset '{asset}' (import it first)")));
        }
    }
    p.compositions[&cid].tracks[ti].clips.push(clip);
    sort_track(p, &cid, ti);
    assign_missing_ids(p);
    Ok(id)
}

/// Remove any object by id (clip, track, effect, marker, asset, composition).
pub fn remove(p: &mut Project, id: &str) -> Result<String> {
    let r = p.find(id).ok_or_else(|| EditError::NotFound(id.to_string()))?;
    let kind = r.kind().to_string();
    match r {
        ObjectRef::Comp(c) => {
            if c == p.root {
                return Err(EditError::Invalid("cannot remove the root composition".into()));
            }
            p.compositions.shift_remove(&c);
        }
        ObjectRef::Asset(a) => {
            p.assets.shift_remove(&a);
        }
        ObjectRef::Track(t) => {
            p.compositions[&t.comp].tracks.remove(t.track);
        }
        ObjectRef::Clip(c) => {
            p.compositions[&c.comp].tracks[c.track].clips.remove(c.clip);
        }
        ObjectRef::Effect(e) => {
            p.effect_list_mut(&e.owner).unwrap().remove(e.index);
        }
        ObjectRef::Marker { comp, index } => {
            p.compositions[&comp].markers.remove(index);
        }
    }
    Ok(kind)
}

/// Split a clip at timeline time `t`. Returns the id of the new right-hand clip.
pub fn split_clip(p: &mut Project, id: &str, t: f64) -> Result<String> {
    let loc = p.find_clip(id).ok_or_else(|| EditError::NotFound(id.to_string()))?;
    let clip = p.clip_at(&loc).unwrap().clone();
    if t <= clip.start + 1e-6 || t >= clip.end() - 1e-6 {
        return Err(EditError::Invalid(format!(
            "split time {t} is outside clip {id} ({}..{})",
            clip.start,
            clip.end()
        )));
    }
    let local = t - clip.start;
    let new_id = p.new_id("c");
    let mut right = clip.clone();
    right.id = new_id.clone();
    right.start = t;
    right.duration = clip.end() - t;
    right.transition_in = None;
    if let Some(remap) = &mut right.time_remap {
        remap.shift_time(-local);
    } else if !matches!(clip.speed, Property::Static(_)) {
        // freeze the variable-speed curve into a time remap to keep continuity
        right.source_in = clip.source_in + clip.integrate_speed(local);
        right.speed.shift_time(-local);
    } else {
        right.source_in = clip.source_in + clip.integrate_speed(local);
    }
    shift_clip_keyframes(&mut right, -local);
    for e in &mut right.effects {
        e.id = String::new();
    }
    let left = p.clip_at_mut(&loc).unwrap();
    left.duration = local;
    p.compositions[&loc.comp].tracks[loc.track].clips.push(right);
    sort_track(p, &loc.comp, loc.track);
    assign_missing_ids(p);
    Ok(new_id)
}

/// Shift every keyframe inside a clip (transform, opacity, effect params...) by `dt`.
pub fn shift_clip_keyframes(clip: &mut Clip, dt: f64) {
    let mut v = serde_json::to_value(&*clip).unwrap();
    shift_keyframes_json(&mut v, dt);
    if let Ok(c) = serde_json::from_value(v) {
        *clip = c;
    }
}

fn shift_keyframes_json(v: &mut Json, dt: f64) {
    match v {
        Json::Object(m) => {
            if let Some(Json::Array(keys)) = m.get_mut("keyframes") {
                for k in keys {
                    match k {
                        Json::Object(km) => {
                            if let Some(t) = km.get("t").and_then(|t| t.as_f64()) {
                                km.insert("t".into(), (t + dt).into());
                            }
                        }
                        Json::Array(a) => {
                            if let Some(t) = a.first().and_then(|t| t.as_f64()) {
                                a[0] = (t + dt).into();
                            }
                        }
                        _ => {}
                    }
                }
            }
            for (k, child) in m.iter_mut() {
                if k != "keyframes" && k != "time_remap" {
                    shift_keyframes_json(child, dt);
                }
            }
        }
        Json::Array(a) => a.iter_mut().for_each(|c| shift_keyframes_json(c, dt)),
        _ => {}
    }
}

/// Move a clip to a new start time and/or track.
pub fn move_clip(p: &mut Project, id: &str, start: Option<f64>, track: Option<&str>) -> Result<()> {
    let loc = p.find_clip(id).ok_or_else(|| EditError::NotFound(id.to_string()))?;
    let mut clip = p.compositions[&loc.comp].tracks[loc.track].clips.remove(loc.clip);
    if let Some(s) = start {
        clip.start = s.max(0.0);
    }
    let ti = match track {
        Some(t) => match p.compositions[&loc.comp].tracks.iter().position(|tr| tr.id == t) {
            Some(i) => i,
            None => {
                p.compositions[&loc.comp].tracks[loc.track].clips.insert(loc.clip, clip);
                return Err(EditError::NotFound(format!("track '{t}'")));
            }
        },
        None => loc.track,
    };
    p.compositions[&loc.comp].tracks[ti].clips.push(clip);
    sort_track(p, &loc.comp, ti);
    Ok(())
}

/// Duplicate a clip (optionally at a new start). Returns the new id.
pub fn duplicate_clip(p: &mut Project, id: &str, start: Option<f64>) -> Result<String> {
    let loc = p.find_clip(id).ok_or_else(|| EditError::NotFound(id.to_string()))?;
    let mut c = p.clip_at(&loc).unwrap().clone();
    c.id = p.new_id("c");
    if let Some(s) = start {
        c.start = s;
    } else {
        c.start = c.end();
    }
    for e in &mut c.effects {
        e.id = String::new();
    }
    let nid = c.id.clone();
    p.compositions[&loc.comp].tracks[loc.track].clips.push(c);
    sort_track(p, &loc.comp, loc.track);
    assign_missing_ids(p);
    Ok(nid)
}

/// Add an effect to a clip, track or composition. Returns the effect instance id.
pub fn add_effect(p: &mut Project, target: &str, mut fx: EffectInstance, index: Option<usize>) -> Result<String> {
    let owner = match p.find(target).ok_or_else(|| EditError::NotFound(target.to_string()))? {
        ObjectRef::Comp(c) => EffectOwner::Comp(c),
        ObjectRef::Track(t) => EffectOwner::Track(t),
        ObjectRef::Clip(c) => EffectOwner::Clip(c),
        other => return Err(EditError::Invalid(format!("cannot add effects to a {}", other.kind()))),
    };
    if fx.id.is_empty() || p.id_exists(&fx.id) {
        fx.id = p.new_id("fx");
    }
    let id = fx.id.clone();
    let list = p.effect_list_mut(&owner).unwrap();
    let i = index.unwrap_or(list.len()).min(list.len());
    list.insert(i, fx);
    Ok(id)
}

/// Move an effect within its stack.
pub fn reorder_effect(p: &mut Project, id: &str, new_index: usize) -> Result<()> {
    let loc = match p.find(id) {
        Some(ObjectRef::Effect(e)) => e,
        _ => return Err(EditError::NotFound(format!("effect '{id}'"))),
    };
    let list = p.effect_list_mut(&loc.owner).unwrap();
    let fx = list.remove(loc.index);
    let i = new_index.min(list.len());
    list.insert(i, fx);
    Ok(())
}

pub fn add_marker(p: &mut Project, comp: Option<&str>, mut m: Marker) -> Result<String> {
    let cid = comp_id(p, comp)?;
    if m.id.is_empty() {
        m.id = p.new_id("m");
    }
    let id = m.id.clone();
    let markers = &mut p.compositions[&cid].markers;
    markers.push(m);
    markers.sort_by(|a, b| a.t.total_cmp(&b.t));
    Ok(id)
}

/// Keep clips on a track ordered by start time.
pub fn sort_track(p: &mut Project, comp: &str, track: usize) {
    if let Some(t) = p.compositions.get_mut(comp).and_then(|c| c.tracks.get_mut(track)) {
        t.clips.sort_by(|a, b| a.start.total_cmp(&b.start));
    }
}

/// Give ids to effects/markers that don't have one.
pub fn assign_missing_ids(p: &mut Project) {
    let mut missing = 0usize;
    for comp in p.compositions.values() {
        missing += comp.effects.iter().filter(|e| e.id.is_empty()).count();
        missing += comp.markers.iter().filter(|e| e.id.is_empty()).count();
        for t in &comp.tracks {
            missing += t.effects.iter().filter(|e| e.id.is_empty()).count();
            for c in &t.clips {
                missing += c.effects.iter().filter(|e| e.id.is_empty()).count();
            }
        }
    }
    if missing == 0 {
        return;
    }
    let mut ids: Vec<String> = (0..missing).map(|_| String::new()).collect();
    for id in &mut ids {
        *id = p.new_id("fx");
        // reserve: temporarily bump; uniqueness is guaranteed by the counter
    }
    let mut it = ids.into_iter();
    for comp in p.compositions.values_mut() {
        for e in comp.effects.iter_mut().filter(|e| e.id.is_empty()) {
            e.id = it.next().unwrap();
        }
        for m in comp.markers.iter_mut().filter(|e| e.id.is_empty()) {
            m.id = it.next().unwrap().replacen("fx", "m", 1);
        }
        for t in &mut comp.tracks {
            for e in t.effects.iter_mut().filter(|e| e.id.is_empty()) {
                e.id = it.next().unwrap();
            }
            for c in &mut t.clips {
                for e in c.effects.iter_mut().filter(|e| e.id.is_empty()) {
                    e.id = it.next().unwrap();
                }
            }
        }
    }
}

/// Snap a time to the nearest beat (if within `tolerance` seconds).
pub fn snap_to_beat(p: &Project, t: f64, tolerance: f64) -> f64 {
    p.timing
        .beats_abs()
        .min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
        .filter(|b| (b - t).abs() <= tolerance)
        .unwrap_or(t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn proj() -> Project {
        Project::new("t", 1280, 720, 30.0, 10.0)
    }

    #[test]
    fn add_split_move_remove() {
        let mut p = proj();
        let c = add_clip(
            &mut p,
            None,
            TrackTarget::New,
            json!({"source": {"type": "solid", "color": "red"}, "duration": 4,
                   "transform": {"scale": {"keyframes": [[0, 1], [4, 2]]}}}),
        )
        .unwrap();
        let r = split_clip(&mut p, &c, 1.0).unwrap();
        let right = p.clip(&r).unwrap();
        assert_eq!(right.start, 1.0);
        assert_eq!(right.duration, 3.0);
        // keyframes shifted: value at local 0 of right == value at local 1 of original
        let s = right.transform.scale.sample(0.0, &crate::math::Vec2::ONE);
        assert!((s.x() - 1.25).abs() < 1e-9);
        move_clip(&mut p, &r, Some(5.0), None).unwrap();
        assert_eq!(p.clip(&r).unwrap().start, 5.0);
        remove(&mut p, &c).unwrap();
        assert!(p.clip(&c).is_none());
    }

    #[test]
    fn paths_and_keyframes() {
        let mut p = proj();
        let c = add_clip(&mut p, None, TrackTarget::New, json!({"source": {"type": "adjustment"}})).unwrap();
        let fx = add_effect(&mut p, &c, EffectInstance::new("", "glow"), None).unwrap();
        set_path(&mut p, &c, &format!("effects.{fx}.params.intensity"), json!(2.0)).unwrap();
        assert_eq!(get_path(&p, &fx, "params.intensity").unwrap(), json!(2.0));
        add_keyframe(&mut p, &c, "opacity", 0.0, Value::Num(0.0), Easing::EaseOut).unwrap();
        add_keyframe(&mut p, &c, "opacity", 1.0, Value::Num(1.0), Easing::Linear).unwrap();
        let clip = p.clip(&c).unwrap();
        assert!(clip.opacity.is_animated());
        merge_object(&mut p, &c, &json!({"blend_mode": "screen"})).unwrap();
        assert_eq!(p.clip(&c).unwrap().blend_mode, crate::model::BlendMode::Screen);
        set_expression(&mut p, &c, "transform.rotation", Some("t * 90.0".into())).unwrap();
        assert!(p.clip(&c).unwrap().transform.rotation.has_expr());
    }

    #[test]
    fn invalid_patch_rejected() {
        let mut p = proj();
        let c = add_clip(&mut p, None, TrackTarget::New, json!({"source": {"type": "adjustment"}})).unwrap();
        assert!(merge_object(&mut p, &c, &json!({"blend_mode": "nope"})).is_err());
        assert_eq!(p.clip(&c).unwrap().blend_mode, crate::model::BlendMode::Normal);
    }
}
