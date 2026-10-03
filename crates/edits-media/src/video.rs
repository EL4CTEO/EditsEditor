//! Streaming video decoder over an FFmpeg pipe with a background prefetch thread.
//!
//! - Sequential access (export, playback) streams frames ahead on a worker thread, so decoding
//!   overlaps GPU rendering.
//! - Short forward jumps are served by reading ahead; backward or far jumps restart FFmpeg with
//!   an accurate input seek.
//! - Output is constant frame rate (`fps` filter) so frame index = round(t * fps) is exact even
//!   for variable-frame-rate sources (phone videos, screen recordings).
//! - Decoded frames go into a small LRU so transitions / echoes / freeze frames that request the
//!   same frame twice don't decode twice. Buffers are recycled to avoid allocation churn.

use std::{
    io::Read,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::{Child, Stdio},
    sync::Arc,
    thread::JoinHandle,
};

use crossbeam_channel::{Receiver, Sender, bounded};
use lru::LruCache;

use crate::{Frame, MediaError, Result, ffmpeg::Ffmpeg};

/// Forward distance (in frames) under which we read ahead instead of seeking.
const READ_AHEAD_LIMIT: u64 = 72;
const PREFETCH: usize = 6;
const CACHE_FRAMES: usize = 24;

struct Stream {
    child: Child,
    rx: Receiver<Option<Vec<u8>>>,
    recycle: Sender<Vec<u8>>,
    /// Index of the next frame that will come out of `rx`.
    next_index: u64,
    reader: Option<JoinHandle<()>>,
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // dropping the receiver makes the reader's `send` fail, so it exits promptly
        let (_tx, dummy) = bounded(0);
        drop(std::mem::replace(&mut self.rx, dummy));
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
    }
}

#[derive(Clone, Debug)]
pub struct VideoOptions {
    /// Output size (scaled by FFmpeg). `None` = native.
    pub size: Option<(u32, u32)>,
    /// Frame rate to resample to. `None` = source rate.
    pub fps: Option<f64>,
    /// Hardware decode accelerator (`auto`, `d3d11va`, `cuda`...). `None` = software.
    pub hwaccel: Option<String>,
}

pub struct VideoReader {
    ff: Ffmpeg,
    path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration: f64,
    frame_bytes: usize,
    hwaccel: Option<String>,
    stream: Option<Stream>,
    cache: LruCache<u64, Arc<Frame>>,
    pub stats: DecodeStats,
}

#[derive(Clone, Debug, Default)]
pub struct DecodeStats {
    pub seeks: u64,
    pub frames_decoded: u64,
    pub cache_hits: u64,
}

impl VideoReader {
    pub fn open(ff: &Ffmpeg, path: &Path, native: (u32, u32), native_fps: f64, duration: f64, opts: VideoOptions) -> Result<VideoReader> {
        let (mut w, mut h) = opts.size.unwrap_or(native);
        // yuv420 scaling wants even sizes
        w = (w.max(2) + 1) & !1;
        h = (h.max(2) + 1) & !1;
        let fps = opts.fps.unwrap_or(native_fps).clamp(1.0, 240.0);
        Ok(VideoReader {
            ff: ff.clone(),
            path: path.to_path_buf(),
            width: w,
            height: h,
            fps,
            duration,
            frame_bytes: (w * h * 4) as usize,
            hwaccel: opts.hwaccel,
            stream: None,
            cache: LruCache::new(NonZeroUsize::new(CACHE_FRAMES).unwrap()),
            stats: DecodeStats::default(),
        })
    }

    pub fn frame_count(&self) -> u64 {
        ((self.duration * self.fps).floor() as u64).max(1)
    }

    pub fn index_for(&self, t: f64) -> u64 {
        let i = (t.max(0.0) * self.fps + 1e-4).floor() as u64;
        i.min(self.frame_count().saturating_sub(1))
    }

    /// Get the frame at source time `t` (seconds).
    pub fn frame_at(&mut self, t: f64) -> Result<Arc<Frame>> {
        let idx = self.index_for(t);
        self.frame(idx)
    }

    pub fn frame(&mut self, idx: u64) -> Result<Arc<Frame>> {
        if let Some(f) = self.cache.get(&idx) {
            self.stats.cache_hits += 1;
            return Ok(f.clone());
        }
        let need_restart = match &self.stream {
            Some(s) => idx < s.next_index || idx - s.next_index > READ_AHEAD_LIMIT,
            None => true,
        };
        if need_restart {
            self.start(idx)?;
        }
        loop {
            let s = self.stream.as_mut().unwrap();
            let got = s.rx.recv().map_err(|_| MediaError::Decode("decoder thread ended".into()))?;
            let cur = s.next_index;
            match got {
                Some(buf) => {
                    s.next_index += 1;
                    self.stats.frames_decoded += 1;
                    let frame = Arc::new(Frame::new(self.width, self.height, buf, false));
                    // keep only recent frames; recycle evicted buffers
                    if let Some((_, old)) = self.cache.push(cur, frame.clone())
                        && let Ok(old) = Arc::try_unwrap(old)
                    {
                        let _ = self.stream.as_ref().unwrap().recycle.try_send(old.data);
                    }
                    if cur >= idx {
                        return Ok(frame);
                    }
                }
                None => {
                    // EOF before reaching idx: return the last frame we have (hold)
                    if let Some((_, f)) = self.cache.iter().max_by_key(|(k, _)| **k) {
                        return Ok(f.clone());
                    }
                    if idx > 0 {
                        // try one frame earlier (durations are often slightly optimistic)
                        self.stream = None;
                        return self.frame(idx.saturating_sub(2));
                    }
                    return Err(MediaError::Decode(format!("no frames decoded from {}", self.path.display())));
                }
            }
        }
    }

    fn start(&mut self, idx: u64) -> Result<()> {
        self.stream = None;
        self.stats.seeks += 1;
        let t = idx as f64 / self.fps;
        let mut cmd = self.ff.cmd();
        if let Some(hw) = &self.hwaccel {
            cmd.args(["-hwaccel", hw]);
        }
        if t > 0.0 {
            cmd.args(["-ss", &format!("{t:.6}")]);
        }
        cmd.arg("-i").arg(&self.path);
        let vf = format!("fps={:.6},scale={}:{}:flags=bicubic,format=rgba", self.fps, self.width, self.height);
        cmd.args(["-an", "-sn", "-dn", "-vf", &vf, "-f", "rawvideo", "-pix_fmt", "rgba", "-"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());
        let mut child = cmd.spawn()?;
        let mut stdout = child.stdout.take().unwrap();
        let (tx, rx) = bounded::<Option<Vec<u8>>>(PREFETCH);
        let (rtx, rrx) = bounded::<Vec<u8>>(PREFETCH + CACHE_FRAMES);
        let n = self.frame_bytes;
        let reader = std::thread::Builder::new()
            .name("edits-video-decode".into())
            .spawn(move || {
                loop {
                    let mut buf = rrx.try_recv().unwrap_or_else(|_| vec![0u8; n]);
                    buf.resize(n, 0);
                    if stdout.read_exact(&mut buf).is_err() {
                        let _ = tx.send(None);
                        return;
                    }
                    if tx.send(Some(buf)).is_err() {
                        return;
                    }
                }
            })
            .map_err(MediaError::Io)?;
        self.stream = Some(Stream { child, rx, recycle: rtx, next_index: idx, reader: Some(reader) });
        Ok(())
    }

    /// Drop the decoder process (frees resources; reopened lazily).
    pub fn close(&mut self) {
        self.stream = None;
    }
}

/// Grab a single frame quickly (thumbnails, contact sheets).
pub fn grab_frame(ff: &Ffmpeg, path: &Path, t: f64, width: u32, height: u32) -> Result<Frame> {
    let w = (width.max(2) + 1) & !1;
    let h = (height.max(2) + 1) & !1;
    let mut cmd = ff.cmd();
    cmd.args(["-ss", &format!("{:.6}", t.max(0.0))]).arg("-i").arg(path);
    cmd.args([
        "-frames:v",
        "1",
        "-an",
        "-vf",
        &format!("scale={w}:{h}:flags=bicubic,format=rgba"),
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgba",
        "-",
    ]);
    let out = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).output()?;
    let n = (w * h * 4) as usize;
    if out.stdout.len() < n {
        return Err(MediaError::Decode(format!(
            "could not grab frame at {t:.2}s from {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(Frame::new(w, h, out.stdout[..n].to_vec(), false))
}
