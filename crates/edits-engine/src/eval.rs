//! Frame evaluation: walks compositions → tracks → clips at a time `t` and drives the GPU
//! compositor (placement, effects, masks, mattes, transitions, echoes, nested comps).

use std::{cell::RefCell, collections::HashSet, sync::Arc};

use edits_core::{
    BlendMode, Clip, ClipSource, Color, Composition, EffectInstance, EndBehavior, Fit, MaskMode, MaskShape, Project, Property,
    TransformState, Value, Vec2,
};
use edits_fx::{EffectKind, Library, ParamType};
use edits_render::{EffectCall, FrameCtx, Globals, MaskParams, Tex, TextFrameParams, TextRenderer, shape, transform};

use crate::{
    EngineError, Result,
    expr::{ClipEval, ExprCtx, ExprEngine, TimingData},
    media_pool::{MediaPool, key_of},
};

const MAX_DEPTH: u32 = 12;

/// A rendered layer with its pending coverage (applied at composite time).
pub struct Layer {
    pub tex: Tex,
    pub opacity: f32,
    pub mask: Option<Tex>,
    pub matte: Option<(Tex, edits_core::MatteMode)>,
}

pub struct EvalCtx<'a> {
    pub project: &'a Project,
    pub lib: &'a Library,
    pub media: &'a mut MediaPool,
    pub text: &'a TextRenderer,
    pub expr: &'a ExprEngine,
    pub timing: Arc<TimingData>,
    pub vars: Arc<indexmap::IndexMap<String, Value>>,
    pub warnings: RefCell<Vec<String>>,
    pub expr_errors: std::rc::Rc<RefCell<Vec<String>>>,
    /// Root timeline time of the frame.
    pub root_time: f64,
}

fn seed_of(a: &str, b: &str) -> u64 {
    key_of((a, b))
}

impl<'a> EvalCtx<'a> {
    pub fn warn(&self, msg: impl Into<String>) {
        let m = msg.into();
        let mut w = self.warnings.borrow_mut();
        if !w.contains(&m) && w.len() < 50 {
            w.push(m);
        }
    }

    fn clip_eval(&self, comp: &Composition, clip: &Clip, comp_t: f64) -> ClipEval<'a> {
        ClipEval {
            engine: self.expr,
            ctx: ExprCtx {
                comp_time: comp_t,
                root_time: self.root_time,
                clip_start: clip.start,
                duration: clip.duration,
                fps: comp.fps,
                width: comp.width as f64,
                height: comp.height as f64,
                seed: seed_of(&clip.id, ""),
                timing: self.timing.clone(),
                variables: self.vars.clone(),
            },
            errors: self.expr_errors.clone(),
        }
    }

    fn comp_eval(&self, comp: &Composition, comp_t: f64) -> ClipEval<'a> {
        ClipEval {
            engine: self.expr,
            ctx: ExprCtx {
                comp_time: comp_t,
                root_time: self.root_time,
                clip_start: 0.0,
                duration: comp.duration,
                fps: comp.fps,
                width: comp.width as f64,
                height: comp.height as f64,
                seed: 0,
                timing: self.timing.clone(),
                variables: self.vars.clone(),
            },
            errors: self.expr_errors.clone(),
        }
    }

    pub fn globals(&self, comp: &Composition, comp_t: f64, local: f64, duration: f64, seed: u64) -> Globals {
        let rt = self.root_time;
        let tm = &self.timing;
        let (beat_index, beat_time) = match tm.beats.partition_point(|b| *b <= rt + 1e-9) {
            0 => (-1.0, 1e6),
            i => ((i - 1) as f32, (rt - tm.beats[i - 1]) as f32),
        };
        let bands = tm.bands(rt);
        Globals {
            time: comp_t as f32,
            local_time: local as f32,
            progress: if duration > 0.0 { (local / duration).clamp(0.0, 1.0) as f32 } else { 0.0 },
            duration: duration as f32,
            frame: (comp_t * comp.fps).floor() as f32,
            seed: (seed % 10_000) as f32 / 10_000.0,
            beat_time,
            beat_index,
            bpm: tm.bpm as f32,
            audio_level: tm.level(rt),
            audio_bands: bands,
            ..Default::default()
        }
    }

    /// Render a composition at comp time `t`. Returns a working texture of the comp size.
    pub fn render_comp(&mut self, f: &mut FrameCtx, comp_id: &str, t: f64, depth: u32) -> Result<Tex> {
        let comp = self.project.compositions.get(comp_id).ok_or_else(|| EngineError::NotFound(format!("composition '{comp_id}'")))?;
        let (w, h) = (comp.width, comp.height);
        let mut acc = f.target(w, h);
        f.clear(acc, comp.background.to_linear_premul());
        if depth > MAX_DEPTH {
            self.warn("nested composition depth exceeded");
            return Ok(acc);
        }
        let any_solo = comp.tracks.iter().any(|t| t.solo);
        for (ti, track) in comp.tracks.iter().enumerate() {
            if !track.enabled || (any_solo && !track.solo) {
                continue;
            }
            let ev = self.comp_eval(comp, t);
            let t_opacity = track.opacity.eval(t, &ev, &1.0) as f32;
            drop(ev);
            if t_opacity <= 0.0 {
                continue;
            }
            let needs_buffer = !track.effects.is_empty() || track.blend_mode != BlendMode::Normal || t_opacity < 0.999;
            let mut target = if needs_buffer { f.target(w, h) } else { acc };
            target = self.render_track(f, comp, ti, t, target, depth)?;
            if needs_buffer {
                let mut buf = target;
                if !track.effects.is_empty() {
                    let ev = self.comp_eval(comp, t);
                    let out = self.apply_effects(f, comp, &track.effects, buf, &ev, t, t, comp.duration, seed_of(&track.id, "fx"))?;
                    drop(ev);
                    if out != buf {
                        f.release(buf);
                        buf = out;
                    }
                }
                acc = f.composite(acc, buf, track.blend_mode, t_opacity, None, None)?;
                f.release(buf);
            } else {
                acc = target;
            }
        }
        if !comp.effects.is_empty() {
            let ev = self.comp_eval(comp, t);
            let out = self.apply_effects(f, comp, &comp.effects, acc, &ev, t, t, comp.duration, seed_of(comp_id, "fx"))?;
            drop(ev);
            if out != acc {
                f.release(acc);
                acc = out;
            }
        }
        Ok(acc)
    }

    fn transition_window(clip: &Clip) -> Option<(f64, f64)> {
        clip.transition_in.as_ref().filter(|tr| tr.duration > 0.0).map(|tr| {
            let start = if tr.centered { clip.start - tr.duration / 2.0 } else { clip.start };
            (start, start + tr.duration)
        })
    }

    fn render_track(&mut self, f: &mut FrameCtx, comp: &Composition, ti: usize, t: f64, mut target: Tex, depth: u32) -> Result<Tex> {
        let track = &comp.tracks[ti];
        // clips being transitioned away from are drawn by the incoming clip
        let mut skip: HashSet<&str> = HashSet::new();
        let mut in_transition: Vec<(usize, usize)> = vec![];
        for (ci, clip) in track.clips.iter().enumerate() {
            if !clip.enabled {
                continue;
            }
            if let Some((a, b)) = Self::transition_window(clip) {
                if t >= a && t < b {
                    if let Some(pi) = track
                        .clips
                        .iter()
                        .enumerate()
                        .filter(|(j, c)| *j != ci && c.enabled && c.start < clip.start)
                        .max_by(|x, y| x.1.end().total_cmp(&y.1.end()))
                        .map(|(j, _)| j)
                    {
                        skip.insert(track.clips[pi].id.as_str());
                        in_transition.push((ci, pi));
                    } else {
                        in_transition.push((ci, usize::MAX));
                    }
                }
            }
        }
        for (ci, clip) in track.clips.iter().enumerate() {
            if !clip.enabled || clip.hidden || skip.contains(clip.id.as_str()) {
                continue;
            }
            let trans = in_transition.iter().find(|(c, _)| *c == ci).copied();
            let visible = clip.is_active(t) || trans.is_some();
            if !visible {
                continue;
            }
            if matches!(clip.source, ClipSource::Adjustment) {
                target = self.apply_adjustment(f, comp, clip, t, target)?;
                continue;
            }
            match trans {
                Some((_, pi)) => {
                    let tr = clip.transition_in.as_ref().unwrap();
                    let (a, b) = Self::transition_window(clip).unwrap();
                    let progress = tr.ease.apply(((t - a) / (b - a)).clamp(0.0, 1.0), b - a);
                    let from = if pi != usize::MAX { self.baked_layer(f, comp, &track.clips[pi], t, depth)? } else { None };
                    let to = self.baked_layer(f, comp, clip, t, depth)?;
                    let (w, h) = (comp.width, comp.height);
                    let from_t = match from {
                        Some(x) => x,
                        None => f.target(w, h),
                    };
                    let to_t = match to {
                        Some(x) => x,
                        None => f.target(w, h),
                    };
                    let out = match self.lib.effect(&tr.effect) {
                        Some(def) if def.kind == EffectKind::Transition => {
                            let ev = self.clip_eval(comp, clip, t);
                            let local = t - a;
                            let params = self.eval_params(def, &tr.params, &ev, local);
                            drop(ev);
                            let mut g = self.globals(comp, t, local, b - a, seed_of(&clip.id, "transition"));
                            g.progress = progress as f32;
                            f.effect(EffectCall { def, params: &params, globals: g, input: Some(from_t), input2: Some(to_t), extra: None, size: (w, h) })?
                        }
                        _ => {
                            self.warn(format!("clip {}: unknown transition '{}' (falling back to crossfade)", clip.id, tr.effect));
                            f.mix(from_t, to_t, progress as f32)?
                        }
                    };
                    f.release(from_t);
                    f.release(to_t);
                    target = f.composite(target, out, clip.blend_mode, 1.0, None, None)?;
                    f.release(out);
                }
                None => {
                    if let Some(layer) = self.clip_layer(f, comp, clip, t, depth)? {
                        target = f.composite(target, layer.tex, clip.blend_mode, layer.opacity, layer.mask, layer.matte)?;
                        f.release(layer.tex);
                        if let Some(m) = layer.mask {
                            f.release(m);
                        }
                        if let Some((m, _)) = layer.matte {
                            f.release(m);
                        }
                    }
                }
            }
        }
        Ok(target)
    }

    /// A layer with coverage baked in (used for transitions and mattes).
    pub fn baked_layer(&mut self, f: &mut FrameCtx, comp: &Composition, clip: &Clip, t: f64, depth: u32) -> Result<Option<Tex>> {
        let Some(layer) = self.clip_layer(f, comp, clip, t, depth)? else { return Ok(None) };
        if layer.opacity >= 0.999 && layer.mask.is_none() && layer.matte.is_none() {
            return Ok(Some(layer.tex));
        }
        let out = f.target(comp.width, comp.height);
        let out = f.composite(out, layer.tex, BlendMode::Normal, layer.opacity, layer.mask, layer.matte)?;
        f.release(layer.tex);
        if let Some(m) = layer.mask {
            f.release(m);
        }
        if let Some((m, _)) = layer.matte {
            f.release(m);
        }
        Ok(Some(out))
    }

    fn apply_adjustment(&mut self, f: &mut FrameCtx, comp: &Composition, clip: &Clip, t: f64, target: Tex) -> Result<Tex> {
        let local = t - clip.start;
        let ev = self.clip_eval(comp, clip, t);
        let opacity = clip.opacity.eval(local, &ev, &1.0) as f32;
        drop(ev);
        if opacity <= 0.0 || clip.effects.is_empty() {
            return Ok(target);
        }
        let ev = self.clip_eval(comp, clip, t);
        let processed = self.apply_effects(f, comp, &clip.effects, target, &ev, t, local, clip.duration, seed_of(&clip.id, "fx"))?;
        drop(ev);
        if processed == target {
            return Ok(target);
        }
        let mask = self.build_mask(f, comp, clip, t)?;
        if opacity >= 0.999 && mask.is_none() {
            f.release(target);
            return Ok(processed);
        }
        // result ≈ target * (1 - k) + processed * k: erase the covered area, then add processed
        let erased = f.composite(target, processed, BlendMode::SilhouetteAlpha, opacity, mask, None)?;
        let out = f.composite(erased, processed, BlendMode::Add, opacity, mask, None)?;
        f.release(processed);
        if let Some(m) = mask {
            f.release(m);
        }
        Ok(out)
    }

    /// Evaluate a clip's transform at a clip-local time.
    fn transform_at(&self, clip: &Clip, ev: &ClipEval, local: f64) -> TransformState {
        let tr = &clip.transform;
        TransformState {
            position: tr.position.eval(local, ev, &Vec2::ZERO),
            anchor: tr.anchor.eval(local, ev, &Vec2::ZERO),
            scale: tr.scale.eval(local, ev, &Vec2::ONE),
            rotation: tr.rotation.eval(local, ev, &0.0),
            rotation_x: tr.rotation_x.eval(local, ev, &0.0),
            rotation_y: tr.rotation_y.eval(local, ev, &0.0),
            z: tr.z.eval(local, ev, &0.0),
            skew: tr.skew.eval(local, ev, &0.0),
        }
    }

    /// Map clip-local time to source time for a source of `dur` seconds (None = still).
    fn source_time(clip: &Clip, local: f64, dur: Option<f64>) -> Option<f64> {
        let s = clip.source_time(local, dur);
        let Some(d) = dur.filter(|d| *d > 0.0) else { return Some(s.max(0.0)) };
        match clip.end_behavior {
            EndBehavior::Hold => Some(s.clamp(0.0, (d - 1e-3).max(0.0))),
            EndBehavior::Loop => Some(s.rem_euclid(d)),
            EndBehavior::PingPong => {
                let p = s.rem_euclid(2.0 * d);
                Some(if p > d { 2.0 * d - p } else { p })
            }
            EndBehavior::Transparent => (s >= 0.0 && s < d).then_some(s),
        }
    }

    /// Render a clip's visual content placed into a comp-sized canvas (no effects).
    fn clip_content(&mut self, f: &mut FrameCtx, comp: &Composition, clip: &Clip, t: f64, local: f64, depth: u32) -> Result<Option<Tex>> {
        let (cw, ch) = (comp.width, comp.height);
        let ev = self.clip_eval(comp, clip, t);
        // ---- source texture ----
        let mut tint = [1.0f32; 4];
        let mut fit = clip.fit;
        let mut owned_src = false;
        let (src, src_size): (Tex, (u32, u32)) = match &clip.source {
            ClipSource::Media { asset, no_video, .. } => {
                if *no_video {
                    return Ok(None);
                }
                let Some(a) = self.project.assets.get(asset) else {
                    self.warn(format!("clip {}: missing asset '{asset}'", clip.id));
                    return Ok(None);
                };
                if matches!(a.kind, edits_core::AssetKind::Audio | edits_core::AssetKind::Font | edits_core::AssetKind::Lut | edits_core::AssetKind::Subtitles | edits_core::AssetKind::Data) {
                    return Ok(None);
                }
                let Some(st) = Self::source_time(clip, local, MediaPool::duration(a)) else { return Ok(None) };
                // SVGs rasterize at their displayed size for crisp vectors
                let raster = if a.kind == edits_core::AssetKind::Svg {
                    let info = a.info.clone().unwrap_or_default();
                    let (bw, bh) = transform::fitted_size((info.width.max(1), info.height.max(1)), (cw, ch), fit);
                    let s = self.transform_at(clip, &ev, local).scale;
                    Some(((bw * s.x().abs()).ceil() as u32, (bh * s.y().abs()).ceil() as u32))
                } else {
                    None
                };
                let vis = match self.media.visual(a, st, (cw, ch), raster) {
                    Ok(v) => v,
                    Err(e) => {
                        self.warn(format!("clip {}: {e}", clip.id));
                        return Ok(None);
                    }
                };
                let tex = f.upload(vis.key, &vis.frame);
                // logical size = asset native size (decode may be downscaled)
                let size = a.info.as_ref().filter(|i| i.width > 0).map(|i| (i.width, i.height)).unwrap_or((vis.frame.width, vis.frame.height));
                (tex, size)
            }
            ClipSource::Solid { color, size } => {
                let c: Color = color.eval(local, &ev, &Color::WHITE);
                tint = c.to_linear_premul();
                let white = edits_media::Frame::solid(1, 1, [255, 255, 255, 255]);
                let tex = f.upload(Some(key_of("white1x1")), &white);
                match size {
                    Some(s) => {
                        fit = Fit::None;
                        (tex, (s.x().max(1.0) as u32, s.y().max(1.0) as u32))
                    }
                    None => {
                        fit = Fit::Stretch;
                        (tex, (cw, ch))
                    }
                }
            }
            ClipSource::Text(src) => {
                let color = src.color.eval(local, &ev, &Color::WHITE);
                let spacing = src.letter_spacing.eval(local, &ev, &0.0);
                fit = Fit::None;
                let animated = TextRenderer::is_animated(src);
                let key = (!animated).then(|| key_of(("text", serde_json::to_string(src).unwrap_or_default(), color.to_hex(), spacing.to_bits())));
                let tex = match key.and_then(|k| f.cached(k)) {
                    Some(t) => t,
                    None => {
                        let frame = self.text.render(src, &TextFrameParams { color, letter_spacing: spacing, time: local, duration: clip.duration });
                        f.upload(key, &frame)
                    }
                };
                let size = f.size(tex);
                (tex, size)
            }
            ClipSource::Shape(src) => {
                fit = Fit::None;
                let trim = src.trim.as_ref().map(|tr| (tr.start.eval(local, &ev, &0.0), tr.end.eval(local, &ev, &1.0), tr.offset.eval(local, &ev, &0.0)));
                let key = key_of(("shape", serde_json::to_string(src).unwrap_or_default(), trim.map(|t| (t.0.to_bits(), t.1.to_bits(), t.2.to_bits()))));
                let tex = match f.cached(key) {
                    Some(t) => t,
                    None => {
                        let frame = shape::render_shape(src, trim);
                        f.upload(Some(key), &frame)
                    }
                };
                let size = f.size(tex);
                (tex, size)
            }
            ClipSource::Generator { effect, params } => {
                let Some(def) = self.lib.effect(effect) else {
                    self.warn(format!("clip {}: unknown generator '{effect}'", clip.id));
                    return Ok(None);
                };
                if def.kind != EffectKind::Generator {
                    self.warn(format!("clip {}: '{effect}' is a {} not a generator (use it in effects instead)", clip.id, def.kind.as_str()));
                }
                let slots = self.eval_params(def, params, &ev, local);
                let g = self.globals(comp, t, local, clip.duration, seed_of(&clip.id, effect));
                let out = f.effect(EffectCall { def, params: &slots, globals: g, input: None, input2: None, extra: None, size: (cw, ch) })?;
                fit = Fit::Stretch;
                owned_src = true;
                (out, (cw, ch))
            }
            ClipSource::Comp { comp: inner } => {
                let Some(ic) = self.project.compositions.get(inner) else {
                    self.warn(format!("clip {}: unknown composition '{inner}'", clip.id));
                    return Ok(None);
                };
                let Some(nt) = Self::source_time(clip, local, Some(ic.duration)) else { return Ok(None) };
                let out = self.render_comp(f, inner, nt, depth + 1)?;
                owned_src = true;
                (out, (ic.width, ic.height))
            }
            ClipSource::Adjustment => return Ok(None),
        };
        // ---- transform + motion blur ----
        let mut times = vec![local];
        if clip.motion_blur && comp.motion_blur.samples > 1 {
            let n = comp.motion_blur.samples.min(64) as usize;
            let shutter = comp.motion_blur.shutter_angle / 360.0 / comp.fps;
            times = (0..n).map(|i| local + (i as f64 / (n - 1) as f64 - 0.5) * shutter).collect();
        }
        let flip = (clip.transform.flip_x, clip.transform.flip_y);
        let samples: Vec<_> = times
            .iter()
            .map(|lt| {
                let st = self.transform_at(clip, &ev, *lt);
                transform::place_params(src_size, (cw, ch), fit, clip.crop, &st, flip, clip.transform.perspective, tint)
            })
            .collect();
        drop(ev);
        // cull layers entirely outside the frame
        let visible = samples.iter().any(|p| match transform::screen_bounds(p) {
            Some([x0, y0, x1, y1]) => x1 > 0.0 && y1 > 0.0 && x0 < cw as f32 && y0 < ch as f32 && (x1 - x0) > 0.01 && (y1 - y0) > 0.01,
            None => false,
        });
        if !visible {
            if owned_src {
                f.release(src);
            }
            return Ok(None);
        }
        // identity fast path: generator / nested comp already in comp space
        let identity = owned_src && samples.len() == 1 && clip.crop.is_none() && is_identity(&self.transform_at_static(clip)) && src_size == (cw, ch);
        if identity {
            return Ok(Some(src));
        }
        let canvas = f.target(cw, ch);
        f.place(canvas, src, &samples)?;
        if owned_src {
            f.release(src);
        }
        Ok(Some(canvas))
    }

    fn transform_at_static(&self, clip: &Clip) -> Option<TransformState> {
        let tr = &clip.transform;
        let all_static = !tr.position.is_animated()
            && !tr.anchor.is_animated()
            && !tr.scale.is_animated()
            && !tr.rotation.is_animated()
            && !tr.rotation_x.is_animated()
            && !tr.rotation_y.is_animated()
            && !tr.z.is_animated()
            && !tr.skew.is_animated();
        all_static.then(|| TransformState {
            position: tr.position.sample(0.0, &Vec2::ZERO),
            anchor: tr.anchor.sample(0.0, &Vec2::ZERO),
            scale: tr.scale.sample(0.0, &Vec2::ONE),
            rotation: tr.rotation.sample(0.0, &0.0),
            rotation_x: tr.rotation_x.sample(0.0, &0.0),
            rotation_y: tr.rotation_y.sample(0.0, &0.0),
            z: tr.z.sample(0.0, &0.0),
            skew: tr.skew.sample(0.0, &0.0),
        })
    }

    /// Full clip layer: content (+ echoes) → effects, with opacity/mask/matte pending.
    pub fn clip_layer(&mut self, f: &mut FrameCtx, comp: &Composition, clip: &Clip, t: f64, depth: u32) -> Result<Option<Layer>> {
        let local = t - clip.start;
        let ev = self.clip_eval(comp, clip, t);
        let opacity = clip.opacity.eval(local, &ev, &1.0) as f32;
        drop(ev);
        if opacity <= 0.0005 {
            return Ok(None);
        }
        let Some(mut tex) = self.clip_content(f, comp, clip, t, local, depth)? else { return Ok(None) };
        // echoes (temporal trails)
        if let Some(echo) = &clip.echo {
            let n = echo.count.clamp(1, 32);
            let mut acc = f.target(comp.width, comp.height);
            if !echo.behind {
                acc = f.composite(acc, tex, BlendMode::Normal, 1.0, None, None)?;
            }
            for k in (1..=n).rev() {
                let lt = local - echo.interval * k as f64;
                if let Some(e) = self.clip_content(f, comp, clip, clip.start + lt, lt, depth)? {
                    let e = self.apply_effects_clip(f, comp, clip, e, t - echo.interval * k as f64, lt)?;
                    acc = f.composite(acc, e, echo.blend, echo.decay.powi(k as i32) as f32, None, None)?;
                    f.release(e);
                }
            }
            if echo.behind {
                acc = f.composite(acc, tex, BlendMode::Normal, 1.0, None, None)?;
            }
            f.release(tex);
            tex = acc;
        }
        let tex = self.apply_effects_clip(f, comp, clip, tex, t, local)?;
        let mask = self.build_mask(f, comp, clip, t)?;
        let matte = match &clip.matte {
            Some(m) if depth < MAX_DEPTH => match self.project.clip(&m.clip) {
                Some(mc) if mc.id != clip.id => {
                    let mcomp = self.project.comp_of(&mc.id).unwrap_or(comp);
                    self.baked_layer(f, mcomp, mc, t, depth + 1)?.map(|tx| (tx, m.mode))
                }
                _ => {
                    self.warn(format!("clip {}: matte clip '{}' not found", clip.id, m.clip));
                    None
                }
            },
            _ => None,
        };
        Ok(Some(Layer { tex, opacity, mask, matte }))
    }

    fn apply_effects_clip(&mut self, f: &mut FrameCtx, comp: &Composition, clip: &Clip, tex: Tex, t: f64, local: f64) -> Result<Tex> {
        if clip.effects.is_empty() {
            return Ok(tex);
        }
        let ev = self.clip_eval(comp, clip, t);
        let out = self.apply_effects(f, comp, &clip.effects, tex, &ev, t, local, clip.duration, seed_of(&clip.id, "fx"))?;
        drop(ev);
        if out != tex {
            f.release(tex);
        }
        Ok(out)
    }

    /// Evaluate effect parameters into uniform slots.
    pub fn eval_params(&self, def: &edits_fx::EffectDef, params: &indexmap::IndexMap<String, Property<Value>>, ev: &ClipEval, local: f64) -> Vec<[f32; 4]> {
        for k in params.keys() {
            if def.param(k).is_none() {
                self.warn(format!(
                    "effect '{}' has no param '{k}' (params: {})",
                    def.id,
                    def.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")
                ));
            }
        }
        def.slot_params()
            .map(|p| {
                let v = params.get(&p.name).map(|prop| prop.eval(local, ev, &p.default)).unwrap_or_else(|| p.default.clone());
                p.to_slot(&v)
            })
            .collect()
    }

    /// Apply an effect stack. Returns the final texture (== input if nothing applied).
    #[allow(clippy::too_many_arguments)]
    pub fn apply_effects(
        &mut self,
        f: &mut FrameCtx,
        comp: &Composition,
        list: &[EffectInstance],
        input: Tex,
        ev: &ClipEval,
        comp_t: f64,
        local: f64,
        duration: f64,
        seed: u64,
    ) -> Result<Tex> {
        let mut cur = input;
        let size = f.size(input);
        for inst in list {
            if !inst.enabled {
                continue;
            }
            if let Some([a, b]) = inst.range {
                if local < a || local > b {
                    continue;
                }
            }
            let Some(def) = self.lib.effect(&inst.effect) else {
                self.warn(format!("unknown effect '{}' (see effects_list)", inst.effect));
                continue;
            };
            if def.kind != EffectKind::Filter {
                self.warn(format!("effect '{}' is a {} — use it as a {}", def.id, def.kind.as_str(), if def.kind == EffectKind::Transition { "transition_in" } else { "generator clip" }));
                continue;
            }
            let mix = inst.mix.eval(local, ev, &1.0) as f32;
            if mix <= 0.0005 {
                continue;
            }
            let mut slots = self.eval_params(def, &inst.params, ev, local);
            // texture parameter (image / LUT)
            let mut extra = None;
            if let Some(tp) = def.texture_param() {
                let id = inst.params.get(&tp.name).map(|p| p.eval(local, ev, &tp.default)).and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                if let Some(a) = self.project.assets.get(&id) {
                    match tp.ty {
                        ParamType::Lut => match self.media.lut(a) {
                            Ok(l) => {
                                extra = Some(f.upload_f32(key_of(("lut", &a.path)), l.width(), l.height(), &l.data));
                                if let Some(i) = def.slot_params().position(|p| p.name == "size") {
                                    slots[i] = [l.size as f32, 0.0, 0.0, 0.0];
                                }
                            }
                            Err(e) => self.warn(format!("effect {}: {e}", def.id)),
                        },
                        _ => match self.media.visual(a, local.max(0.0), size, None) {
                            Ok(v) => extra = Some(f.upload(v.key, &v.frame)),
                            Err(e) => self.warn(format!("effect {}: {e}", def.id)),
                        },
                    }
                } else if !id.is_empty() {
                    self.warn(format!("effect {}: asset '{id}' not found", def.id));
                }
            }
            let g = self.globals(comp, comp_t, local, duration, seed ^ key_of(&inst.id));
            let out = f.effect(EffectCall { def, params: &slots, globals: g, input: Some(cur), input2: None, extra, size })?;
            let out = if mix < 0.9995 {
                let m = f.mix(cur, out, mix)?;
                f.release(out);
                m
            } else {
                out
            };
            if cur != input {
                f.release(cur);
            }
            cur = out;
        }
        Ok(cur)
    }

    /// Rasterize a clip's masks into one coverage texture.
    pub fn build_mask(&mut self, f: &mut FrameCtx, comp: &Composition, clip: &Clip, t: f64) -> Result<Option<Tex>> {
        if clip.masks.is_empty() {
            return Ok(None);
        }
        let local = t - clip.start;
        let ev = self.clip_eval(comp, clip, t);
        let mut cur: Option<Tex> = None;
        for m in &clip.masks {
            let feather = m.feather.eval(local, &ev, &0.0) as f32;
            let expansion = m.expansion.eval(local, &ev, &0.0) as f32;
            let opacity = m.opacity.eval(local, &ev, &1.0) as f32;
            let offset = m.offset.eval(local, &ev, &Vec2::ZERO);
            let rotation = m.rotation.eval(local, &ev, &0.0).to_radians() as f32;
            let scale = m.scale.eval(local, &ev, &Vec2::ONE);
            let (shape, center, size, radius, points) = match &m.shape {
                MaskShape::Rect { center, size, radius } => {
                    let c = center.eval(local, &ev, &Vec2::ZERO);
                    let s = size.eval(local, &ev, &Vec2::new(comp.width as f64 / 2.0, comp.height as f64 / 2.0));
                    (0u32, [c.x() as f32, c.y() as f32], [s.x() as f32, s.y() as f32], radius.eval(local, &ev, &0.0) as f32, vec![])
                }
                MaskShape::Ellipse { center, size } => {
                    let c = center.eval(local, &ev, &Vec2::ZERO);
                    let s = size.eval(local, &ev, &Vec2::new(comp.height as f64 / 2.0, comp.height as f64 / 2.0));
                    (1, [c.x() as f32, c.y() as f32], [s.x() as f32, s.y() as f32], 0.0, vec![])
                }
                MaskShape::Polygon { points } => (2, [0.0, 0.0], [0.0, 0.0], 0.0, points.iter().map(|p| [p.x() as f32, p.y() as f32]).collect()),
                MaskShape::Path { d } => (2, [0.0, 0.0], [0.0, 0.0], 0.0, shape::flatten_svg(d, 64)),
            };
            let mode = match m.mode {
                MaskMode::Add => 0,
                MaskMode::Subtract => 1,
                MaskMode::Intersect => 2,
                MaskMode::Difference => 3,
            };
            let params = MaskParams {
                shape,
                mode,
                invert: m.invert,
                center,
                size,
                radius,
                feather,
                expansion,
                opacity,
                offset: [offset.x() as f32, offset.y() as f32],
                rotation,
                scale: [scale.x() as f32, scale.y() as f32],
                points,
            };
            cur = Some(f.mask(cur, (comp.width, comp.height), &params)?);
        }
        Ok(cur)
    }
}

fn is_identity(st: &Option<TransformState>) -> bool {
    match st {
        Some(s) => *s == TransformState::default(),
        None => false,
    }
}
