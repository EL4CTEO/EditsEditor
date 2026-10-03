//! Shot / scene detection and per-shot statistics, computed from a tiny low-res decode.
//! Robust to anime-typical white flash frames and fades.

use std::{io::Read, path::Path, process::Stdio};

use serde::{Deserialize, Serialize};

use crate::{Result, ffmpeg::Ffmpeg};

const W: usize = 64;
const H: usize = 36;
const BINS: usize = 8 * 4 * 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shot {
    pub index: usize,
    pub start: f64,
    pub end: f64,
    /// Mean frame-to-frame change inside the shot (0..1): high = action, low = static.
    pub motion: f64,
    /// Mean luminance 0..1.
    pub brightness: f64,
    /// Mean saturation 0..1.
    pub saturation: f64,
    /// Average color "#rrggbb".
    pub color: String,
    /// Suggested representative time (sharpest / most typical frame).
    pub thumb: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneOptions {
    /// Cut sensitivity 0..1 (higher = more cuts).
    pub sensitivity: f64,
    /// Minimum shot length in seconds.
    pub min_shot: f64,
    /// Analysis frame rate (lower = faster). 0 = source rate (max 30).
    pub sample_fps: f64,
    /// Analyze only [start, end] seconds of the source.
    pub range: Option<[f64; 2]>,
}

impl Default for SceneOptions {
    fn default() -> Self {
        SceneOptions { sensitivity: 0.5, min_shot: 0.35, sample_fps: 0.0, range: None }
    }
}

struct FrameStat {
    hist: [f32; BINS],
    pixels: Vec<u8>,
    luma: f32,
    sat: f32,
    rgb: [f32; 3],
}

fn stat(buf: &[u8]) -> FrameStat {
    let mut hist = [0f32; BINS];
    let mut luma = 0.0;
    let mut sat = 0.0;
    let mut rgb = [0f32; 3];
    let n = (W * H) as f32;
    for px in buf.chunks_exact(3) {
        let (r, g, b) = (px[0] as usize, px[1] as usize, px[2] as usize);
        hist[(r >> 5) * 16 + (g >> 6) * 4 + (b >> 6)] += 1.0;
        let (rf, gf, bf) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        luma += 0.2126 * rf + 0.7152 * gf + 0.0722 * bf;
        let mx = rf.max(gf).max(bf);
        let mn = rf.min(gf).min(bf);
        sat += if mx > 0.0 { (mx - mn) / mx } else { 0.0 };
        rgb[0] += rf;
        rgb[1] += gf;
        rgb[2] += bf;
    }
    for h in &mut hist {
        *h /= n;
    }
    FrameStat { hist, pixels: buf.to_vec(), luma: luma / n, sat: sat / n, rgb: rgb.map(|c| c / n) }
}

fn diff(a: &FrameStat, b: &FrameStat) -> f32 {
    let hd: f32 = a.hist.iter().zip(b.hist.iter()).map(|(x, y)| (x - y).abs()).sum::<f32>() * 0.5;
    let pd: f32 = a.pixels.iter().zip(b.pixels.iter()).map(|(x, y)| (*x as i32 - *y as i32).unsigned_abs()).sum::<u32>() as f32
        / (a.pixels.len() as f32 * 255.0);
    0.55 * hd + 0.45 * (pd * 2.5).min(1.0)
}

/// Detect shots in a video file.
pub fn detect_scenes(ff: &Ffmpeg, path: &Path, src_fps: f64, opts: &SceneOptions) -> Result<Vec<Shot>> {
    let fps = if opts.sample_fps > 0.0 { opts.sample_fps } else { src_fps.min(30.0) }.max(1.0);
    let mut cmd = ff.cmd();
    let offset = opts.range.map(|r| r[0]).unwrap_or(0.0);
    if offset > 0.0 {
        cmd.args(["-ss", &format!("{offset:.3}")]);
    }
    cmd.arg("-i").arg(path);
    if let Some(r) = opts.range {
        cmd.args(["-t", &format!("{:.3}", (r[1] - r[0]).max(0.1))]);
    }
    cmd.args([
        "-an",
        "-sn",
        "-vf",
        &format!("fps={fps},scale={W}:{H}:flags=area,format=rgb24"),
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-",
    ]);
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let mut out = child.stdout.take().unwrap();
    let mut stats = vec![];
    let mut buf = vec![0u8; W * H * 3];
    while out.read_exact(&mut buf).is_ok() {
        stats.push(stat(&buf));
    }
    let _ = child.wait();
    if stats.is_empty() {
        return Ok(vec![]);
    }
    let n = stats.len();
    let d: Vec<f32> = (0..n).map(|i| if i == 0 { 0.0 } else { diff(&stats[i - 1], &stats[i]) }).collect();
    // adaptive threshold: base + k * local median
    let base = 0.55 - opts.sensitivity as f32 * 0.4;
    let min_gap = (opts.min_shot * fps).ceil() as usize;
    let mut cuts = vec![0usize];
    for i in 1..n {
        let lo = i.saturating_sub(12);
        let hi = (i + 12).min(n);
        let mut win: Vec<f32> = d[lo..hi].to_vec();
        win.sort_by(|a, b| a.total_cmp(b));
        let med = win[win.len() / 2];
        let thr = base.max(med * 3.0 + 0.05);
        if d[i] > thr && i - *cuts.last().unwrap() >= min_gap {
            // flash frame guard: if the frame after next is similar to the frame before, skip
            let is_flash = i + 2 < n && diff(&stats[i - 1], &stats[i + 2]) < thr * 0.5 && stats[i].luma > stats[i - 1].luma + 0.25;
            if !is_flash {
                cuts.push(i);
            }
        }
    }
    cuts.push(n);
    let mut shots = vec![];
    for (si, w) in cuts.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        if b <= a {
            continue;
        }
        let len = (b - a) as f64;
        let motion = if b - a > 1 { d[a + 1..b].iter().map(|x| *x as f64).sum::<f64>() / (len - 1.0).max(1.0) } else { 0.0 };
        let brightness = stats[a..b].iter().map(|s| s.luma as f64).sum::<f64>() / len;
        let saturation = stats[a..b].iter().map(|s| s.sat as f64).sum::<f64>() / len;
        let mut rgb = [0f64; 3];
        for s in &stats[a..b] {
            for c in 0..3 {
                rgb[c] += s.rgb[c] as f64 / len;
            }
        }
        // representative frame: least change from neighbors in the middle 60%
        let ia = a + (b - a) / 5;
        let ib = (b - (b - a) / 5).max(ia + 1).min(b);
        let thumb_i = (ia..ib).min_by(|x, y| d[*x].total_cmp(&d[*y])).unwrap_or(a);
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        shots.push(Shot {
            index: si,
            start: offset + a as f64 / fps,
            end: offset + b as f64 / fps,
            motion: (motion * 4.0).min(1.0),
            brightness,
            saturation,
            color: format!("#{:02x}{:02x}{:02x}", c(rgb[0]), c(rgb[1]), c(rgb[2])),
            thumb: offset + thumb_i as f64 / fps,
        });
    }
    Ok(shots)
}
