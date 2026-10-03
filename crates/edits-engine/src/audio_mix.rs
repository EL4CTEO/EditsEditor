//! Builds the audio mix of a composition (including nested compositions).

use std::{cell::RefCell, sync::Arc};

use edits_audio::{AudioFx, MixClip};
use edits_core::{AudioEffect, Clip, ClipSource, EndBehavior, Project};
use edits_media::AudioBuffer;

use crate::{
    Result,
    engine::Engine,
    expr::{ClipEval, ExprCtx, ExprEngine, TimingData},
    media_pool::{MediaPool, key_of},
};

type TimeMap = Arc<dyn Fn(f64) -> f64 + Send + Sync>;

struct Ctx {
    project: Arc<Project>,
    expr: Arc<ExprEngine>,
    timing: Arc<TimingData>,
    vars: Arc<indexmap::IndexMap<String, edits_core::Value>>,
}

fn eval_f64(ctx: &Ctx, clip: &Clip, fps: f64, prop: &edits_core::Property<f64>, local: f64, comp_t: f64, default: f64) -> f64 {
    if !prop.has_expr() {
        return prop.sample(local, &default);
    }
    let errors = std::rc::Rc::new(RefCell::new(vec![]));
    let ev = ClipEval {
        engine: &ctx.expr,
        ctx: ExprCtx {
            comp_time: comp_t,
            root_time: comp_t,
            clip_start: clip.start,
            duration: clip.duration,
            fps,
            width: 0.0,
            height: 0.0,
            seed: key_of((&clip.id, "")),
            timing: ctx.timing.clone(),
            variables: ctx.vars.clone(),
        },
        errors,
    };
    prop.eval(local, &ev, &default)
}

fn source_time_audio(clip: &Clip, local: f64, dur: Option<f64>) -> f64 {
    if clip.freeze.is_some() {
        return -1.0;
    }
    let mut s = clip.source_time(local, dur);
    // stutter: repeat a slice inside the window
    for fx in &clip.audio.effects {
        if let AudioEffect::Stutter { start, end, slice } = fx
            && local >= *start
            && local < *end
            && *slice > 0.0
        {
            let into = (local - start).rem_euclid(*slice);
            s = clip.source_time(*start + into, dur);
        }
    }
    match dur {
        Some(d) if d > 0.0 => match clip.end_behavior {
            EndBehavior::Loop => s.rem_euclid(d),
            EndBehavior::PingPong => {
                let p = s.rem_euclid(2.0 * d);
                if p > d { 2.0 * d - p } else { p }
            }
            _ => {
                if s >= d {
                    -1.0
                } else {
                    s
                }
            }
        },
        _ => s,
    }
}

impl Engine {
    /// Mix a composition's audio over [t0, t1).
    pub fn mix_audio(&mut self, comp: Option<&str>, t0: f64, t1: f64) -> Result<AudioBuffer> {
        let comp_id = comp.map(String::from).unwrap_or_else(|| self.project.root.clone());
        let ctx = Ctx {
            project: Arc::new(self.project.clone()),
            expr: self.expr.clone(),
            timing: self.timing_data(),
            vars: Arc::new(self.project.variables.clone()),
        };
        let mut clips: Vec<MixClip<'static>> = vec![];
        let identity: TimeMap = Arc::new(|t| t);
        self.collect_audio(&ctx, &comp_id, identity, 1.0, 0, &mut clips)?;
        Ok(edits_audio::mix(&clips, t0, t1))
    }

    /// Does this composition contain any audible clips?
    pub fn has_audio(&self, comp: Option<&str>) -> bool {
        let id = comp.unwrap_or(&self.project.root);
        let Some(c) = self.project.compositions.get(id) else { return false };
        c.tracks.iter().any(|t| {
            !t.muted
                && t.clips.iter().any(|cl| match &cl.source {
                    ClipSource::Media { asset, no_audio, .. } => {
                        !no_audio
                            && !cl.audio.mute
                            && self.project.assets.get(asset).and_then(|a| a.info.as_ref()).is_some_and(|i| i.has_audio)
                    }
                    ClipSource::Comp { comp } => self.has_audio(Some(comp)),
                    _ => false,
                })
        })
    }

    /// `to_comp`: maps root timeline time → this comp's time. `gain`: accumulated parent gain.
    fn collect_audio(
        &mut self,
        ctx: &Ctx,
        comp_id: &str,
        to_comp: TimeMap,
        gain: f64,
        depth: u32,
        out: &mut Vec<MixClip<'static>>,
    ) -> Result<()> {
        if depth > 8 {
            return Ok(());
        }
        let project = ctx.project.clone();
        let Some(comp) = project.compositions.get(comp_id) else { return Ok(()) };
        let any_solo = comp.tracks.iter().any(|t| t.solo);
        let comp_vol = comp.volume.sample(0.0, &1.0);
        for track in &comp.tracks {
            if !track.enabled || track.muted || (any_solo && !track.solo) {
                continue;
            }
            let track_vol = track.volume.sample(0.0, &1.0);
            for clip in &track.clips {
                if !clip.enabled || clip.audio.mute {
                    continue;
                }
                match &clip.source {
                    ClipSource::Media { asset, no_audio, .. } => {
                        if *no_audio {
                            continue;
                        }
                        let Some(a) = project.assets.get(asset) else { continue };
                        if !a.info.as_ref().is_some_and(|i| i.has_audio) {
                            continue;
                        }
                        let buffer = match self.media.audio(a) {
                            Ok(b) => b,
                            Err(e) => {
                                tracing::warn!("audio for {}: {e}", clip.id);
                                continue;
                            }
                        };
                        let dur = MediaPool::duration(a).or(Some(buffer.duration()));
                        let c = Arc::new(clip.clone());
                        let tc = to_comp.clone();
                        // timeline range of this clip (only exact for linear parent maps)
                        let (start, end) = timeline_range(&to_comp, clip.start, clip.end());
                        let c1 = c.clone();
                        let tc1 = tc.clone();
                        let source_time: Box<dyn Fn(f64) -> f64 + Send + Sync> = Box::new(move |t| {
                            let ct = tc1(t);
                            let local = ct - c1.start;
                            if local < 0.0 || local >= c1.duration {
                                return -1.0;
                            }
                            source_time_audio(&c1, local, dur)
                        });
                        let c2 = c.clone();
                        let tc2 = tc.clone();
                        let ctx2 = Ctx {
                            project: ctx.project.clone(),
                            expr: ctx.expr.clone(),
                            timing: ctx.timing.clone(),
                            vars: ctx.vars.clone(),
                        };
                        let fps = comp.fps;
                        let g0 = gain * track_vol * comp_vol;
                        let gain_fn: Box<dyn Fn(f64) -> f64 + Send + Sync> = Box::new(move |t| {
                            let ct = tc2(t);
                            let local = ct - c2.start;
                            let v = eval_f64(&ctx2, &c2, fps, &c2.audio.volume, local, ct, 1.0).max(0.0);
                            let fi = if c2.audio.fade_in > 0.0 { (local / c2.audio.fade_in).clamp(0.0, 1.0) } else { 1.0 };
                            let fo =
                                if c2.audio.fade_out > 0.0 { ((c2.duration - local) / c2.audio.fade_out).clamp(0.0, 1.0) } else { 1.0 };
                            v * fi * fo * g0
                        });
                        let c3 = c.clone();
                        let tc3 = tc.clone();
                        let pan_fn: Box<dyn Fn(f64) -> f64 + Send + Sync> = Box::new(move |t| c3.audio.pan.sample(tc3(t) - c3.start, &0.0));
                        let mut effects = vec![];
                        for fx in &clip.audio.effects {
                            let cc = c.clone();
                            let tcc = tc.clone();
                            effects.push(match fx {
                                AudioEffect::Lowpass { cutoff, q } => {
                                    let p = cutoff.clone();
                                    AudioFx::Lowpass { cutoff: Box::new(move |t| p.sample(tcc(t) - cc.start, &20000.0)), q: *q }
                                }
                                AudioEffect::Highpass { cutoff, q } => {
                                    let p = cutoff.clone();
                                    AudioFx::Highpass { cutoff: Box::new(move |t| p.sample(tcc(t) - cc.start, &20.0)), q: *q }
                                }
                                AudioEffect::Echo { delay, feedback, mix } => {
                                    AudioFx::Echo { delay: *delay, feedback: *feedback, mix: *mix }
                                }
                                AudioEffect::Distortion { drive } => AudioFx::Distortion { drive: *drive },
                                AudioEffect::Bitcrush { bits, downsample } => AudioFx::Bitcrush { bits: *bits, downsample: *downsample },
                                AudioEffect::Gain { db } => {
                                    let p = db.clone();
                                    AudioFx::Gain { db: Box::new(move |t| p.sample(tcc(t) - cc.start, &0.0)) }
                                }
                                AudioEffect::Stutter { .. } => continue,
                            });
                        }
                        out.push(MixClip { buffer, start, end, source_time, gain: gain_fn, pan: pan_fn, effects });
                    }
                    ClipSource::Comp { comp: inner } => {
                        let Some(ic) = project.compositions.get(inner) else { continue };
                        let c = Arc::new(clip.clone());
                        let parent = to_comp.clone();
                        let idur = ic.duration;
                        let map: TimeMap = Arc::new(move |t| {
                            let local = parent(t) - c.start;
                            if local < 0.0 || local >= c.duration {
                                return -1e9;
                            }
                            c.source_time(local, Some(idur))
                        });
                        let v = clip.audio.volume.sample(0.0, &1.0);
                        self.collect_audio(ctx, inner, map, gain * v * track_vol * comp_vol, depth + 1, out)?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// Approximate root-timeline range in which comp-time interval [a, b) is visible.
fn timeline_range(to_comp: &TimeMap, a: f64, b: f64) -> (f64, f64) {
    // probe the identity case quickly
    if (to_comp(a) - a).abs() < 1e-9 && (to_comp(b) - b).abs() < 1e-9 {
        return (a, b);
    }
    // otherwise scan the root timeline coarsely (nested comps): up to 2 hours at 20ms
    let mut lo = f64::MAX;
    let mut hi = f64::MIN;
    let mut t = 0.0;
    while t < 7200.0 {
        let ct = to_comp(t);
        if ct >= a && ct < b {
            lo = lo.min(t);
            hi = hi.max(t + 0.02);
        } else if hi > lo && ct < -1e8 {
            break;
        }
        t += 0.02;
    }
    if hi > lo { (lo, hi) } else { (0.0, 0.0) }
}
