//! Offline audio mixer: time-mapped (speed ramps, reverse, stutter) clips with gain/pan
//! automation and a per-clip effect chain, summed with a soft limiter.

use std::sync::Arc;

use edits_media::{AudioBuffer, SAMPLE_RATE};

const BLOCK: usize = 64;

pub type Curve<'a> = Box<dyn Fn(f64) -> f64 + Send + Sync + 'a>;

/// A per-clip audio effect with resolved automation.
pub enum AudioFx<'a> {
    Lowpass { cutoff: Curve<'a>, q: f64 },
    Highpass { cutoff: Curve<'a>, q: f64 },
    Echo { delay: f64, feedback: f64, mix: f64 },
    Distortion { drive: f64 },
    Bitcrush { bits: f64, downsample: f64 },
    Gain { db: Curve<'a> },
}

pub struct MixClip<'a> {
    pub buffer: Arc<AudioBuffer>,
    /// Timeline range.
    pub start: f64,
    pub end: f64,
    /// Timeline time → source seconds. Return a negative/NaN value for silence.
    pub source_time: Curve<'a>,
    /// Timeline time → linear gain (volume × fades × track × master).
    pub gain: Curve<'a>,
    /// Timeline time → pan (-1..1).
    pub pan: Curve<'a>,
    pub effects: Vec<AudioFx<'a>>,
}

#[derive(Clone, Copy, Default)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z: [[f64; 2]; 2],
}

impl Biquad {
    fn set(&mut self, low: bool, cutoff: f64, q: f64, sr: f64) {
        let f = cutoff.clamp(10.0, sr * 0.49);
        let w0 = std::f64::consts::TAU * f / sr;
        let (s, c) = w0.sin_cos();
        let alpha = s / (2.0 * q.max(0.05));
        let a0 = 1.0 + alpha;
        let (b0, b1, b2) = if low { ((1.0 - c) / 2.0, 1.0 - c, (1.0 - c) / 2.0) } else { ((1.0 + c) / 2.0, -(1.0 + c), (1.0 + c) / 2.0) };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = -2.0 * c / a0;
        self.a2 = (1.0 - alpha) / a0;
    }
    #[inline]
    fn process(&mut self, ch: usize, x: f64) -> f64 {
        // transposed direct form II
        let y = self.b0 * x + self.z[ch][0];
        self.z[ch][0] = self.b1 * x - self.a1 * y + self.z[ch][1];
        self.z[ch][1] = self.b2 * x - self.a2 * y;
        y
    }
}

fn apply_fx(fx: &AudioFx, data: &mut [f32], t0: f64, sr: f64) {
    let frames = data.len() / 2;
    match fx {
        AudioFx::Lowpass { cutoff, q } | AudioFx::Highpass { cutoff, q } => {
            let low = matches!(fx, AudioFx::Lowpass { .. });
            let mut bq = Biquad::default();
            for b in (0..frames).step_by(BLOCK) {
                bq.set(low, cutoff(t0 + b as f64 / sr), *q, sr);
                for i in b..(b + BLOCK).min(frames) {
                    for ch in 0..2 {
                        data[i * 2 + ch] = bq.process(ch, data[i * 2 + ch] as f64) as f32;
                    }
                }
            }
        }
        AudioFx::Echo { delay, feedback, mix } => {
            let d = ((delay.max(0.001)) * sr) as usize;
            let mut line = vec![0f32; d * 2];
            let mut pos = 0;
            for i in 0..frames {
                for ch in 0..2 {
                    let dry = data[i * 2 + ch];
                    let wet = line[pos * 2 + ch];
                    line[pos * 2 + ch] = dry + wet * *feedback as f32;
                    data[i * 2 + ch] = dry + wet * *mix as f32;
                }
                pos = (pos + 1) % d;
            }
        }
        AudioFx::Distortion { drive } => {
            let k = drive.max(0.01) as f32;
            let norm = k.tanh();
            for s in data.iter_mut() {
                *s = (*s * k).tanh() / norm;
            }
        }
        AudioFx::Bitcrush { bits, downsample } => {
            let levels = 2f32.powf(bits.clamp(1.0, 24.0) as f32);
            let step = downsample.max(1.0) as usize;
            let mut hold = [0f32; 2];
            for i in 0..frames {
                if i % step == 0 {
                    for ch in 0..2 {
                        hold[ch] = (data[i * 2 + ch] * levels).round() / levels;
                    }
                }
                data[i * 2] = hold[0];
                data[i * 2 + 1] = hold[1];
            }
        }
        AudioFx::Gain { db } => {
            for b in (0..frames).step_by(BLOCK) {
                let g = 10f64.powf(db(t0 + b as f64 / sr) / 20.0) as f32;
                for v in &mut data[b * 2..((b + BLOCK).min(frames)) * 2] {
                    *v *= g;
                }
            }
        }
    }
}

/// Render one clip's contribution over its full range into a stereo buffer.
fn render_clip(c: &MixClip, sr: f64) -> (usize, Vec<f32>) {
    let start_f = (c.start * sr).round().max(0.0) as usize;
    let end_f = (c.end * sr).round().max(0.0) as usize;
    let n = end_f.saturating_sub(start_f);
    let mut out = vec![0f32; n * 2];
    let src_sr = c.buffer.sample_rate as f64;
    for b in (0..n).step_by(BLOCK) {
        let e = (b + BLOCK).min(n);
        let ta = (start_f + b) as f64 / sr;
        let tb = (start_f + e) as f64 / sr;
        let (sa, sb) = ((c.source_time)(ta), (c.source_time)(tb));
        let (ga, gb) = ((c.gain)(ta) as f32, (c.gain)(tb) as f32);
        let (pa, pb) = ((c.pan)(ta) as f32, (c.pan)(tb) as f32);
        let len = (e - b) as f64;
        for i in b..e {
            let f = (i - b) as f64 / len;
            let s = sa + (sb - sa) * f;
            if !s.is_finite() || s < 0.0 {
                continue;
            }
            let (l, r) = c.buffer.sample_at(s * src_sr);
            let ff = f as f32;
            let g = ga + (gb - ga) * ff;
            let pan = (pa + (pb - pa) * ff).clamp(-1.0, 1.0);
            let lg = (1.0 - pan).min(1.0);
            let rg = (1.0 + pan).min(1.0);
            out[i * 2] = l * g * lg;
            out[i * 2 + 1] = r * g * rg;
        }
    }
    for fx in &c.effects {
        apply_fx(fx, &mut out, start_f as f64 / sr, sr);
    }
    (start_f, out)
}

#[inline]
fn soft_limit(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.95 { x } else { x.signum() * (0.95 + 0.05 * ((a - 0.95) / 0.05).tanh()) }
}

/// Mix clips into a stereo buffer covering timeline [t0, t1).
pub fn mix(clips: &[MixClip], t0: f64, t1: f64) -> AudioBuffer {
    use rayon::prelude::*;
    let sr = SAMPLE_RATE as f64;
    let f0 = (t0 * sr).round() as usize;
    let f1 = (t1 * sr).round() as usize;
    let n = f1.saturating_sub(f0);
    let rendered: Vec<(usize, Vec<f32>)> = clips
        .par_iter()
        .filter(|c| c.end > t0 && c.start < t1)
        .map(|c| render_clip(c, sr))
        .collect();
    let mut out = vec![0f32; n * 2];
    for (start, data) in rendered {
        let frames = data.len() / 2;
        for i in 0..frames {
            let gf = start + i;
            if gf < f0 || gf >= f1 {
                continue;
            }
            let o = (gf - f0) * 2;
            out[o] += data[i * 2];
            out[o + 1] += data[i * 2 + 1];
        }
    }
    for s in &mut out {
        *s = soft_limit(*s);
    }
    AudioBuffer { sample_rate: SAMPLE_RATE, channels: 2, samples: out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_and_gain() {
        // 1 second ramp 0..1 on both channels
        let n = SAMPLE_RATE as usize;
        let mut s = vec![0f32; n * 2];
        for i in 0..n {
            s[i * 2] = i as f32 / n as f32;
            s[i * 2 + 1] = s[i * 2];
        }
        let buf = Arc::new(AudioBuffer { sample_rate: SAMPLE_RATE, channels: 2, samples: s });
        let clip = MixClip {
            buffer: buf,
            start: 1.0,
            end: 1.5,
            source_time: Box::new(|t| (t - 1.0) * 2.0),
            gain: Box::new(|_| 0.5),
            pan: Box::new(|_| 0.0),
            effects: vec![],
        };
        let out = mix(&[clip], 0.0, 2.0);
        // at timeline 1.25 -> source 0.5 -> value 0.5 * gain 0.5 = 0.25
        let i = (1.25 * SAMPLE_RATE as f64) as usize;
        assert!((out.samples[i * 2] - 0.25).abs() < 0.01, "{}", out.samples[i * 2]);
        assert_eq!(out.samples[10], 0.0);
    }
}
