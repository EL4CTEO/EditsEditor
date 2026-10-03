//! Music analysis: onset strength, tempo, beat tracking, downbeats, drops, sections, accents,
//! and per-band loudness envelopes (used by audio-reactive expressions).
//!
//! Pipeline: mono downmix → 24 kHz → STFT (1024 / hop 256, Hann) → log-compressed band flux
//! onset envelope → tempo via prior-weighted autocorrelation → dynamic-programming beat tracker
//! (Ellis 2007) → downbeat phase from bass emphasis → drops from energy jumps at downbeats →
//! bar-energy sections.

use std::sync::Arc;

use realfft::RealFftPlanner;
use serde::{Deserialize, Serialize};

use edits_media::AudioBuffer;

const SR: f64 = 24_000.0;
const NFFT: usize = 1024;
const HOP: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisOptions {
    pub min_bpm: f64,
    pub max_bpm: f64,
    /// Known tempo (narrows the search to ±8%).
    pub bpm_hint: Option<f64>,
    /// Beat tracker tightness (higher = steadier grid).
    pub tightness: f64,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        AnalysisOptions { min_bpm: 60.0, max_bpm: 200.0, bpm_hint: None, tightness: 120.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionInfo {
    pub start: f64,
    pub end: f64,
    pub label: String,
    pub energy: f64,
}

/// Loudness envelopes sampled at `rate` Hz.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Envelope {
    pub rate: f64,
    /// Overall loudness 0..~1.
    pub level: Vec<f32>,
    /// [bass, low-mid, high-mid, treble] 0..~1.
    pub bands: Vec<[f32; 4]>,
    /// Onset strength 0..~1 (transient-ness).
    pub onset: Vec<f32>,
}

impl Envelope {
    fn idx(&self, t: f64) -> Option<(usize, usize, f32)> {
        if self.level.is_empty() || t < 0.0 {
            return None;
        }
        let x = t * self.rate;
        let i = x.floor() as usize;
        let n = self.level.len();
        if i >= n {
            return None;
        }
        Some((i, (i + 1).min(n - 1), (x - i as f64) as f32))
    }

    pub fn level_at(&self, t: f64) -> f32 {
        self.idx(t).map(|(a, b, f)| self.level[a] + (self.level[b] - self.level[a]) * f).unwrap_or(0.0)
    }

    pub fn bands_at(&self, t: f64) -> [f32; 4] {
        self.idx(t)
            .map(|(a, b, f)| {
                let mut o = [0.0; 4];
                for (k, v) in o.iter_mut().enumerate() {
                    *v = self.bands[a][k] + (self.bands[b][k] - self.bands[a][k]) * f;
                }
                o
            })
            .unwrap_or([0.0; 4])
    }

    pub fn onset_at(&self, t: f64) -> f32 {
        self.idx(t).map(|(a, b, f)| self.onset[a] + (self.onset[b] - self.onset[a]) * f).unwrap_or(0.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub duration: f64,
    pub bpm: f64,
    /// Confidence of the tempo estimate 0..1.
    pub tempo_confidence: f64,
    pub beats: Vec<f64>,
    pub downbeats: Vec<f64>,
    pub drops: Vec<f64>,
    pub accents: Vec<f64>,
    pub sections: Vec<SectionInfo>,
    pub envelope: Envelope,
}

impl AudioAnalysis {
    /// Index of the last beat at or before `t`, and time since it.
    pub fn beat_before(&self, t: f64) -> Option<(usize, f64)> {
        let i = self.beats.partition_point(|b| *b <= t);
        if i == 0 { None } else { Some((i - 1, t - self.beats[i - 1])) }
    }
}

fn percentile(v: &[f32], p: f64) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    s[((s.len() - 1) as f64 * p).round() as usize]
}

fn smooth(v: &[f32], radius: usize) -> Vec<f32> {
    if radius == 0 || v.is_empty() {
        return v.to_vec();
    }
    let mut prefix = vec![0f64; v.len() + 1];
    for (i, x) in v.iter().enumerate() {
        prefix[i + 1] = prefix[i] + *x as f64;
    }
    (0..v.len())
        .map(|i| {
            let a = i.saturating_sub(radius);
            let b = (i + radius + 1).min(v.len());
            ((prefix[b] - prefix[a]) / (b - a) as f64) as f32
        })
        .collect()
}

/// Analyze a decoded audio buffer.
pub fn analyze(buf: &AudioBuffer, opts: &AnalysisOptions) -> AudioAnalysis {
    let mono48 = buf.mono();
    let ratio = (buf.sample_rate as f64 / SR).round().max(1.0) as usize;
    // box-filter decimation to ~24 kHz, with centered padding for frame alignment
    let mut x: Vec<f32> = vec![0.0; NFFT / 2];
    x.extend(mono48.chunks(ratio).map(|c| c.iter().sum::<f32>() / c.len() as f32));
    x.extend(std::iter::repeat_n(0.0, NFFT));
    let duration = buf.duration();
    let frame_rate = SR / HOP as f64;
    let n_frames = ((x.len() - NFFT) / HOP).max(1);

    // ---- STFT ----
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(NFFT);
    let window: Vec<f32> = (0..NFFT).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / NFFT as f32).cos()).collect();
    let n_bins = NFFT / 2 + 1;
    // log-spaced band edges for flux (32 bands, 40 Hz .. 11 kHz)
    let bin_hz = SR / NFFT as f64;
    let flux_edges: Vec<usize> = (0..=32)
        .map(|i| {
            let f = 40.0 * (11000.0f64 / 40.0).powf(i as f64 / 32.0);
            ((f / bin_hz).round() as usize).clamp(1, n_bins - 1)
        })
        .collect();
    let band_edges = [(20.0, 150.0), (150.0, 600.0), (600.0, 4000.0), (4000.0, 11500.0)]
        .map(|(a, b): (f64, f64)| (((a / bin_hz).floor() as usize).max(1), ((b / bin_hz).ceil() as usize).min(n_bins - 1)));

    let frames: Vec<(Vec<f32>, [f32; 4], f32)> = {
        use rayon::prelude::*;
        (0..n_frames)
            .into_par_iter()
            .map_init(
                || (fft.make_input_vec(), fft.make_output_vec(), fft.make_scratch_vec()),
                |(input, output, scratch), f| {
                    let off = f * HOP;
                    for i in 0..NFFT {
                        input[i] = x[off + i] * window[i];
                    }
                    let _ = fft.process_with_scratch(input, output, scratch);
                    let mag: Vec<f32> = output.iter().map(|c| c.norm()).collect();
                    let mut flux_bands = vec![0f32; 32];
                    for b in 0..32 {
                        let (a, e) = (flux_edges[b], flux_edges[b + 1].max(flux_edges[b] + 1));
                        let s: f32 = mag[a..e.min(n_bins)].iter().sum::<f32>() / (e - a) as f32;
                        flux_bands[b] = (1.0 + 100.0 * s).ln();
                    }
                    let mut bands = [0f32; 4];
                    for (k, (a, e)) in band_edges.iter().enumerate() {
                        let s: f32 = mag[*a..=*e].iter().map(|m| m * m).sum::<f32>() / (e - a + 1) as f32;
                        bands[k] = s.sqrt();
                    }
                    let total: f32 = (mag.iter().map(|m| m * m).sum::<f32>() / n_bins as f32).sqrt();
                    (flux_bands, bands, total)
                },
            )
            .collect()
    };

    // ---- onset envelope ----
    let mut onset = vec![0f32; n_frames];
    for f in 1..n_frames {
        let (a, b) = (&frames[f - 1].0, &frames[f].0);
        onset[f] = a.iter().zip(b.iter()).map(|(p, c)| (c - p).max(0.0)).sum();
    }
    let local = smooth(&onset, 8);
    for (o, l) in onset.iter_mut().zip(local.iter()) {
        *o = (*o - l).max(0.0);
    }
    let mean = onset.iter().sum::<f32>() / n_frames as f32;
    let std = (onset.iter().map(|o| (o - mean).powi(2)).sum::<f32>() / n_frames as f32).sqrt().max(1e-6);
    for o in &mut onset {
        *o /= std;
    }

    // ---- envelopes ----
    let mut env_bands: Vec<[f32; 4]> = frames.iter().map(|f| f.1).collect();
    let mut level: Vec<f32> = frames.iter().map(|f| f.2).collect();
    for k in 0..4 {
        let col: Vec<f32> = env_bands.iter().map(|b| (1.0 + 50.0 * b[k]).ln()).collect();
        let p = percentile(&col, 0.98).max(1e-6);
        let release = (-1.0 / (0.12 * frame_rate)).exp() as f32;
        let mut y = 0f32;
        for (i, c) in col.iter().enumerate() {
            let v = (c / p).min(1.5);
            y = if v > y { v } else { y * release + v * (1.0 - release) };
            env_bands[i][k] = y;
        }
    }
    {
        let col: Vec<f32> = level.iter().map(|l| (1.0 + 50.0 * l).ln()).collect();
        let p = percentile(&col, 0.98).max(1e-6);
        for (l, c) in level.iter_mut().zip(col.iter()) {
            *l = (c / p).min(1.5);
        }
    }
    let onset_p = percentile(&onset, 0.99).max(1e-6);
    let onset_env: Vec<f32> = onset.iter().map(|o| (o / onset_p).min(1.5)).collect();

    // ---- tempo ----
    let (min_bpm, max_bpm) = match opts.bpm_hint {
        Some(h) => (h * 0.92, h * 1.08),
        None => (opts.min_bpm, opts.max_bpm),
    };
    let lag_min = (60.0 * frame_rate / max_bpm).floor().max(2.0) as usize;
    let lag_max = ((60.0 * frame_rate / min_bpm).ceil() as usize).min(n_frames / 2).max(lag_min + 1);
    let ac = |lag: usize| -> f64 {
        if lag >= n_frames {
            return 0.0;
        }
        let s: f64 = (0..n_frames - lag).map(|t| onset[t] as f64 * onset[t + lag] as f64).sum();
        s / (n_frames - lag) as f64
    };
    let acs: Vec<f64> = (0..=lag_max * 2 + 1).map(|l| if l >= lag_min / 2 { ac(l) } else { 0.0 }).collect();
    let mut best = (lag_min, f64::MIN);
    let mut scores = vec![];
    for lag in lag_min..=lag_max {
        let bpm = 60.0 * frame_rate / lag as f64;
        let prior = (-0.5 * ((bpm / 120.0).log2() / 0.9).powi(2)).exp();
        let s = (acs[lag] + 0.5 * acs.get(lag * 2).copied().unwrap_or(0.0) + 0.25 * acs.get(lag / 2).copied().unwrap_or(0.0)) * prior;
        scores.push(s);
        if s > best.1 {
            best = (lag, s);
        }
    }
    // parabolic refinement
    let li = best.0;
    let refine = |l: usize| -> f64 {
        if l <= lag_min || l >= lag_max {
            return l as f64;
        }
        let (a, b, c) = (scores[l - 1 - lag_min], scores[l - lag_min], scores[l + 1 - lag_min]);
        let d = a - 2.0 * b + c;
        if d.abs() < 1e-12 { l as f64 } else { l as f64 + 0.5 * (a - c) / d }
    };
    let period = refine(li).max(1.0);
    let bpm = 60.0 * frame_rate / period;
    let mean_s = scores.iter().sum::<f64>() / scores.len().max(1) as f64;
    let tempo_confidence = if best.1 > 0.0 { (1.0 - mean_s / best.1).clamp(0.0, 1.0) } else { 0.0 };

    // ---- beat tracking (DP) ----
    let mut score = vec![0f64; n_frames];
    let mut back = vec![usize::MAX; n_frames];
    let p = period;
    for t in 0..n_frames {
        let lo = (t as f64 - 2.0 * p).max(0.0) as usize;
        let hi = (t as f64 - p / 2.0).max(0.0) as usize;
        let mut bv = 0.0;
        let mut bi = usize::MAX;
        if hi > lo {
            for prev in lo..hi {
                let r = ((t - prev) as f64 / p).ln();
                let v = score[prev] - opts.tightness * r * r;
                if bi == usize::MAX || v > bv {
                    bv = v;
                    bi = prev;
                }
            }
        }
        score[t] = onset[t] as f64 + if bi == usize::MAX { 0.0 } else { bv.max(0.0) };
        back[t] = if bv > 0.0 { bi } else { usize::MAX };
    }
    let tail = (n_frames as f64 - p).max(0.0) as usize;
    let mut end = (tail..n_frames).max_by(|a, b| score[*a].total_cmp(&score[*b])).unwrap_or(n_frames - 1);
    let mut beat_frames = vec![end];
    while back[end] != usize::MAX {
        end = back[end];
        beat_frames.push(end);
    }
    beat_frames.reverse();
    // extend the grid to the start if tracking began late
    while let Some(&first) = beat_frames.first() {
        let prev = first as f64 - p;
        if prev < 0.0 {
            break;
        }
        beat_frames.insert(0, prev.round() as usize);
    }
    let to_time = |f: usize| f as f64 * HOP as f64 / SR;
    let beats: Vec<f64> = beat_frames.iter().map(|f| to_time(*f)).filter(|t| *t <= duration).map(round3).collect();

    // ---- downbeats ----
    let bass_at = |t: f64| -> f32 {
        // peak raw bass energy shortly after the beat (onsets land slightly late in frames)
        let i = ((t * frame_rate).max(0.0) as usize).min(n_frames - 1);
        let j = (i + (0.09 * frame_rate) as usize).min(n_frames - 1);
        let peak = (i.saturating_sub(1)..=j).map(|k| frames[k].1[0]).fold(0.0f32, f32::max);
        let on = (i.saturating_sub(1)..=j).map(|k| onset_env[k]).fold(0.0f32, f32::max);
        (1.0 + 50.0 * peak).ln() + 0.25 * on
    };
    let phase = (0..4)
        .max_by(|a, b| {
            let sa: f32 = beats.iter().skip(*a).step_by(4).map(|t| bass_at(*t)).sum();
            let sb: f32 = beats.iter().skip(*b).step_by(4).map(|t| bass_at(*t)).sum();
            sa.total_cmp(&sb)
        })
        .unwrap_or(0);
    let downbeats: Vec<f64> = beats.iter().skip(phase).step_by(4).copied().collect();

    // ---- drops ----
    let energy = smooth(&level, (0.25 * frame_rate) as usize);
    let at = |t: f64| ((t * frame_rate).max(0.0) as usize).min(n_frames - 1);
    let mean_range = |a: f64, b: f64| -> f32 {
        let (i, j) = (at(a), at(b).max(at(a) + 1));
        energy[i..j.min(n_frames)].iter().sum::<f32>() / (j.min(n_frames) - i).max(1) as f32
    };
    let e70 = percentile(&energy, 0.7);
    let mut cands: Vec<(f64, f32)> = downbeats
        .iter()
        .filter(|d| **d > 4.0 && **d < duration - 2.0)
        .filter_map(|d| {
            let pre = mean_range(d - 4.0, d - 0.25);
            let post = mean_range(*d, d + 2.0);
            let ratio = post / (pre + 0.05);
            (ratio > 1.35 && post >= e70).then_some((*d, ratio * post))
        })
        .collect();
    cands.sort_by(|a, b| b.1.total_cmp(&a.1));
    let max_drops = ((duration / 25.0).ceil() as usize).max(1);
    let mut drops: Vec<f64> = vec![];
    for (t, _) in cands {
        if drops.len() >= max_drops {
            break;
        }
        if drops.iter().all(|d| (d - t).abs() > 8.0) {
            drops.push(t);
        }
    }
    drops.sort_by(|a, b| a.total_cmp(b));

    // ---- accents (strong onsets) ----
    let thr = percentile(&onset, 0.93);
    let min_gap = (0.12 * frame_rate) as usize;
    let mut accents = vec![];
    let mut last: Option<usize> = None;
    for i in 3..n_frames.saturating_sub(3) {
        let v = onset[i];
        if v >= thr && (i - 3..=i + 3).all(|j| onset[j] <= v) && last.is_none_or(|l| i - l >= min_gap) {
            accents.push(round3(to_time(i)));
            last = Some(i);
        }
    }

    // ---- sections from bar energy ----
    let sections = sections_from(&downbeats, duration, &drops, |a, b| mean_range(a, b) as f64);

    let envelope = Envelope {
        rate: frame_rate,
        level: level.iter().map(|v| (v * 1000.0).round() / 1000.0).collect(),
        bands: env_bands.iter().map(|b| b.map(|v| (v * 1000.0).round() / 1000.0)).collect(),
        onset: onset_env.iter().map(|v| (v * 1000.0).round() / 1000.0).collect(),
    };
    AudioAnalysis {
        duration,
        bpm: (bpm * 100.0).round() / 100.0,
        tempo_confidence,
        beats,
        downbeats,
        drops,
        accents,
        sections,
        envelope,
    }
}

fn round3(t: f64) -> f64 {
    (t * 1000.0).round() / 1000.0
}

fn sections_from(downbeats: &[f64], duration: f64, drops: &[f64], energy: impl Fn(f64, f64) -> f64) -> Vec<SectionInfo> {
    if downbeats.len() < 4 {
        return vec![SectionInfo { start: 0.0, end: duration, label: "full".into(), energy: 1.0 }];
    }
    // group bars in pairs of 2 bars (8 beats) for stability
    let mut bounds: Vec<f64> = vec![0.0];
    bounds.extend(downbeats.iter().skip(2).step_by(2).copied());
    bounds.push(duration);
    bounds.dedup_by(|a, b| (*a - *b).abs() < 0.5);
    let segs: Vec<(f64, f64, f64)> = bounds.windows(2).map(|w| (w[0], w[1], energy(w[0], w[1]))).collect();
    let mut es: Vec<f64> = segs.iter().map(|s| s.2).collect();
    es.sort_by(|a, b| a.total_cmp(b));
    let q = |p: f64| es[((es.len() - 1) as f64 * p) as usize];
    let (q1, q2) = (q(0.33), q(0.7));
    let emax = es.last().copied().unwrap_or(1.0).max(1e-6);
    let level = |e: f64| if e >= q2 { 2 } else if e >= q1 { 1 } else { 0 };
    // merge runs of equal level
    let mut runs: Vec<(f64, f64, i32, f64, usize)> = vec![];
    for (a, b, e) in segs {
        let l = level(e);
        match runs.last_mut() {
            Some(r) if r.2 == l => {
                r.1 = b;
                r.3 += e;
                r.4 += 1;
            }
            _ => runs.push((a, b, l, e, 1)),
        }
    }
    // absorb single short runs into the previous one
    let mut merged: Vec<(f64, f64, i32, f64, usize)> = vec![];
    for r in runs {
        if let Some(last) = merged.last_mut() {
            if r.1 - r.0 < 4.0 {
                last.1 = r.1;
                last.3 += r.3;
                last.4 += r.4;
                continue;
            }
        }
        merged.push(r);
    }
    let n = merged.len();
    merged
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let e = r.3 / r.4 as f64;
            let has_drop = drops.iter().any(|d| *d >= r.0 - 1.0 && *d < r.1);
            let next_high = merged.get(i + 1).is_some_and(|n| n.2 == 2);
            let label = match r.2 {
                2 if has_drop => "drop",
                2 => "chorus",
                1 if next_high => "build",
                1 => "verse",
                _ if i == 0 => "intro",
                _ if i == n - 1 => "outro",
                _ if next_high => "build",
                _ => "break",
            };
            SectionInfo { start: round3(r.0), end: round3(r.1), label: label.into(), energy: (e / emax * 1000.0).round() / 1000.0 }
        })
        .collect()
}

/// Convenience: analyze and wrap in an Arc.
pub fn analyze_arc(buf: &AudioBuffer, opts: &AnalysisOptions) -> Arc<AudioAnalysis> {
    Arc::new(analyze(buf, opts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click_track(bpm: f64, seconds: f64) -> AudioBuffer {
        let sr = 48_000.0;
        let n = (seconds * sr) as usize;
        let mut s = vec![0f32; n * 2];
        let period = 60.0 / bpm;
        let mut k = 0usize;
        loop {
            let t0 = k as f64 * period;
            if t0 >= seconds {
                break;
            }
            let accent = if k % 4 == 0 { 1.0 } else { 0.5 };
            let start = (t0 * sr) as usize;
            for i in 0..2400 {
                if start + i >= n {
                    break;
                }
                let t = i as f64 / sr;
                // kick-ish: low sine with decay + click
                let v = ((t * 2.0 * std::f64::consts::PI * 60.0).sin() * accent + if i < 40 { 0.8 } else { 0.0 }) * (-t * 30.0).exp();
                s[(start + i) * 2] += v as f32;
                s[(start + i) * 2 + 1] += v as f32;
            }
            k += 1;
        }
        AudioBuffer { sample_rate: 48_000, channels: 2, samples: s }
    }

    #[test]
    fn tracks_tempo_and_beats() {
        for bpm in [96.0, 128.0, 140.0] {
            let a = analyze(&click_track(bpm, 30.0), &AnalysisOptions::default());
            assert!((a.bpm - bpm).abs() < 2.0, "expected {bpm}, got {}", a.bpm);
            let period = 60.0 / bpm;
            // beats should land on the grid
            let mut err = 0.0;
            for b in &a.beats {
                let k = (b / period).round();
                err += (b - k * period).abs();
            }
            err /= a.beats.len() as f64;
            assert!(err < 0.03, "mean beat error {err} at {bpm}");
            // downbeats should be on accented beats (multiples of 4 periods)
            let d = a.downbeats[1];
            let k = (d / period).round() as i64;
            assert_eq!(k % 4, 0, "downbeat phase wrong at {bpm}: {d}");
        }
    }
}
