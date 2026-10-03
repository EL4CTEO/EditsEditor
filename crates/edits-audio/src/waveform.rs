//! Waveform overview data (min/max/rms per column) for visualizations.

use edits_media::AudioBuffer;

/// Per-column (min, max, rms) of the mono signal over [t0, t1).
pub fn columns(buf: &AudioBuffer, t0: f64, t1: f64, width: usize) -> Vec<(f32, f32, f32)> {
    let sr = buf.sample_rate as f64;
    let n = buf.frames();
    let a = ((t0 * sr).max(0.0) as usize).min(n);
    let b = ((t1 * sr).max(0.0) as usize).min(n).max(a);
    let per = ((b - a) as f64 / width.max(1) as f64).max(1.0);
    (0..width)
        .map(|x| {
            let s = a + (x as f64 * per) as usize;
            let e = (a + ((x + 1) as f64 * per) as usize).min(b);
            if s >= e {
                return (0.0, 0.0, 0.0);
            }
            let mut mn = f32::MAX;
            let mut mx = f32::MIN;
            let mut sq = 0f64;
            for i in s..e {
                let v = (buf.samples[i * 2] + buf.samples[i * 2 + 1]) * 0.5;
                mn = mn.min(v);
                mx = mx.max(v);
                sq += (v * v) as f64;
            }
            (mn, mx, (sq / (e - s) as f64).sqrt() as f32)
        })
        .collect()
}
