//! Lazily opened, cached media sources for every asset kind.

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

use edits_core::{Asset, AssetKind, MediaInfo};
use edits_fx::lut::Lut3d;
use edits_media::{
    AudioBuffer, Ffmpeg, Frame, VideoOptions, VideoReader,
    image_io::{self, Animation},
    sequence::ImageSequence,
    subtitles::{self, Cue},
};
use lru::LruCache;

use crate::{EngineError, Result};

pub fn key_of(h: impl Hash) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    h.hash(&mut s);
    s.finish()
}

pub struct MediaPool {
    pub ff: Option<Ffmpeg>,
    pub base_dir: PathBuf,
    pub hwaccel: Option<String>,
    videos: LruCache<(PathBuf, u32, u32), VideoReader>,
    images: LruCache<PathBuf, Arc<Frame>>,
    anims: HashMap<PathBuf, Arc<Animation>>,
    sequences: HashMap<PathBuf, Arc<ImageSequence>>,
    seq_frames: LruCache<(PathBuf, usize), Arc<Frame>>,
    svgs: LruCache<(PathBuf, u32, u32), Arc<Frame>>,
    audio: HashMap<PathBuf, Arc<AudioBuffer>>,
    luts: HashMap<PathBuf, Arc<Lut3d>>,
    subs: HashMap<PathBuf, Arc<Vec<Cue>>>,
}

/// A visual frame plus an optional GPU cache key (None = changes every frame).
pub struct Visual {
    pub frame: Arc<Frame>,
    pub key: Option<u64>,
}

impl MediaPool {
    pub fn new(ff: Option<Ffmpeg>, base_dir: PathBuf) -> MediaPool {
        MediaPool {
            hwaccel: None,
            ff,
            base_dir,
            videos: LruCache::new(NonZeroUsize::new(16).unwrap()),
            images: LruCache::new(NonZeroUsize::new(64).unwrap()),
            anims: HashMap::new(),
            sequences: HashMap::new(),
            seq_frames: LruCache::new(NonZeroUsize::new(48).unwrap()),
            svgs: LruCache::new(NonZeroUsize::new(32).unwrap()),
            audio: HashMap::new(),
            luts: HashMap::new(),
            subs: HashMap::new(),
        }
    }

    pub fn resolve(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() || path.contains("://") { p.to_path_buf() } else { self.base_dir.join(p) }
    }

    pub fn ffmpeg(&self) -> Result<&Ffmpeg> {
        self.ff.as_ref().ok_or(EngineError::Media(edits_media::MediaError::FfmpegMissing))
    }

    /// Duration (seconds) of time-based media, None for stills.
    pub fn duration(asset: &Asset) -> Option<f64> {
        match asset.kind {
            AssetKind::Video | AssetKind::Audio | AssetKind::AnimatedImage => asset.info.as_ref().and_then(|i| i.duration),
            AssetKind::ImageSequence => asset.info.as_ref().and_then(|i| i.frames).map(|n| n as f64 / asset.sequence_fps.unwrap_or(24.0)),
            _ => None,
        }
    }

    /// Size to decode a video at, given how big it appears in the composition.
    pub fn decode_size(info: &MediaInfo, comp: (u32, u32)) -> (u32, u32) {
        let (w, h) = (info.width.max(2), info.height.max(2));
        // cover-fit size * 1.5 headroom for zooms, never above native
        let s = (comp.0 as f64 / w as f64).max(comp.1 as f64 / h as f64) * 1.5;
        if s >= 1.0 {
            (w, h)
        } else {
            let even = |v: f64| ((v.round() as u32).max(2) + 1) & !1;
            (even(w as f64 * s), even(h as f64 * s))
        }
    }

    /// Get the visual frame of an asset at source time `t` (already end-behavior mapped).
    pub fn visual(&mut self, asset: &Asset, t: f64, comp: (u32, u32), raster_size: Option<(u32, u32)>) -> Result<Visual> {
        let path = self.resolve(&asset.path);
        match asset.kind {
            AssetKind::Video => {
                let info = asset.info.clone().unwrap_or_default();
                let (w, h) = Self::decode_size(&info, comp);
                let key = (path.clone(), w, h);
                if !self.videos.contains(&key) {
                    let ff = self.ffmpeg()?.clone();
                    let r = VideoReader::open(
                        &ff,
                        &path,
                        (info.width, info.height),
                        info.fps.unwrap_or(24.0),
                        info.duration.unwrap_or(0.0),
                        VideoOptions { size: Some((w, h)), fps: None, hwaccel: self.hwaccel.clone() },
                    )?;
                    self.videos.put(key.clone(), r);
                }
                let reader = self.videos.get_mut(&key).unwrap();
                let frame = reader.frame_at(t)?;
                Ok(Visual { frame, key: None })
            }
            AssetKind::Image => {
                if let Some(f) = self.images.get(&path) {
                    return Ok(Visual { frame: f.clone(), key: Some(key_of(("img", &path))) });
                }
                let f = Arc::new(image_io::load_image(&path)?);
                self.images.put(path.clone(), f.clone());
                Ok(Visual { frame: f, key: Some(key_of(("img", &path))) })
            }
            AssetKind::AnimatedImage => {
                let anim = match self.anims.get(&path) {
                    Some(a) => a.clone(),
                    None => {
                        let a = Arc::new(image_io::load_animation(&path)?);
                        self.anims.insert(path.clone(), a.clone());
                        a
                    }
                };
                let i = anim.frame_at(t);
                Ok(Visual { frame: anim.frames[i].clone(), key: Some(key_of(("anim", &path, i))) })
            }
            AssetKind::Svg => {
                let (w, h) = raster_size.unwrap_or_else(|| asset.info.as_ref().map(|i| (i.width, i.height)).unwrap_or((512, 512)));
                // quantize raster size to reduce re-rasterization during zooms
                let q = |v: u32| (v.clamp(16, 4096).div_ceil(32)) * 32;
                let k = (path.clone(), q(w), q(h));
                if let Some(f) = self.svgs.get(&k) {
                    return Ok(Visual { frame: f.clone(), key: Some(key_of(("svg", &k))) });
                }
                let f = Arc::new(image_io::render_svg(&path, k.1, k.2)?);
                self.svgs.put(k.clone(), f.clone());
                Ok(Visual { frame: f, key: Some(key_of(("svg", &k))) })
            }
            AssetKind::ImageSequence => {
                let seq = match self.sequences.get(&path) {
                    Some(s) => s.clone(),
                    None => {
                        let s = Arc::new(ImageSequence::open(&path.to_string_lossy())?);
                        self.sequences.insert(path.clone(), s.clone());
                        s
                    }
                };
                let fps = asset.sequence_fps.unwrap_or(24.0);
                let i = ((t * fps + 1e-4).floor().max(0.0) as usize).min(seq.len() - 1);
                let k = (path.clone(), i);
                if let Some(f) = self.seq_frames.get(&k) {
                    return Ok(Visual { frame: f.clone(), key: Some(key_of(("seq", &k))) });
                }
                let f = Arc::new(image_io::load_image(&seq.files[i])?);
                self.seq_frames.put(k.clone(), f.clone());
                Ok(Visual { frame: f, key: Some(key_of(("seq", &k))) })
            }
            other => Err(EngineError::Invalid(format!("asset kind {other:?} has no visuals"))),
        }
    }

    pub fn audio(&mut self, asset: &Asset) -> Result<Arc<AudioBuffer>> {
        let path = self.resolve(&asset.path);
        if let Some(a) = self.audio.get(&path) {
            return Ok(a.clone());
        }
        let a = Arc::new(edits_media::decode_audio(self.ffmpeg()?, &path)?);
        self.audio.insert(path, a.clone());
        Ok(a)
    }

    pub fn lut(&mut self, asset: &Asset) -> Result<Arc<Lut3d>> {
        let path = self.resolve(&asset.path);
        if let Some(l) = self.luts.get(&path) {
            return Ok(l.clone());
        }
        let text = std::fs::read_to_string(&path)?;
        let l = Arc::new(Lut3d::parse_cube(&text).map_err(EngineError::Invalid)?);
        self.luts.insert(path, l.clone());
        Ok(l)
    }

    pub fn subtitles(&mut self, asset: &Asset) -> Result<Arc<Vec<Cue>>> {
        let path = self.resolve(&asset.path);
        if let Some(s) = self.subs.get(&path) {
            return Ok(s.clone());
        }
        let text = std::fs::read_to_string(&path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let s = Arc::new(subtitles::parse(&text, ext));
        self.subs.insert(path, s.clone());
        Ok(s)
    }

    /// Close decoders (frees processes) — e.g. after a project switch.
    pub fn close_all(&mut self) {
        self.videos.clear();
    }

    pub fn open_decoders(&self) -> usize {
        self.videos.len()
    }
}
