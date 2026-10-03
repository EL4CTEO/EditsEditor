//! Still images, animated images (GIF / WebP / APNG) and SVG.

use std::{fs::File, io::BufReader, path::Path, sync::Arc};

use image::AnimationDecoder;

use crate::{Frame, MediaError, Result};

pub fn load_image(path: &Path) -> Result<Frame> {
    let img = image::ImageReader::open(path)
        .map_err(|e| MediaError::Decode(format!("{}: {e}", path.display())))?
        .with_guessed_format()
        .map_err(|e| MediaError::Decode(format!("{}: {e}", path.display())))?
        .decode()
        .map_err(|e| MediaError::Decode(format!("{}: {e}", path.display())))?;
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Ok(Frame::new(w, h, rgba.into_raw(), false))
}

/// A fully decoded animation (frames are composited to the full canvas).
pub struct Animation {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<Arc<Frame>>,
    /// Display duration of each frame in seconds.
    pub delays: Vec<f64>,
}

impl Animation {
    pub fn duration(&self) -> f64 {
        self.delays.iter().sum()
    }

    /// Frame index at time `t` (looping is handled by the caller).
    pub fn frame_at(&self, t: f64) -> usize {
        let mut acc = 0.0;
        for (i, d) in self.delays.iter().enumerate() {
            acc += d;
            if t < acc {
                return i;
            }
        }
        self.frames.len().saturating_sub(1)
    }
}

fn collect(frames: Vec<image::Frame>) -> Result<Animation> {
    if frames.is_empty() {
        return Err(MediaError::Decode("animation has no frames".into()));
    }
    let mut out = Vec::with_capacity(frames.len());
    let mut delays = Vec::with_capacity(frames.len());
    let (w, h) = frames[0].buffer().dimensions();
    for f in frames {
        let (n, d) = f.delay().numer_denom_ms();
        let ms = if d == 0 { 100.0 } else { n as f64 / d as f64 };
        // browsers clamp tiny GIF delays to 100ms; do the same for <= 10ms
        delays.push(if ms <= 10.0 { 0.1 } else { ms / 1000.0 });
        let buf = f.into_buffer();
        let (fw, fh) = buf.dimensions();
        out.push(Arc::new(Frame::new(fw, fh, buf.into_raw(), false)));
    }
    Ok(Animation { width: w, height: h, frames: out, delays })
}

pub fn load_animation(path: &Path) -> Result<Animation> {
    let err = |e: image::ImageError| MediaError::Decode(format!("{}: {e}", path.display()));
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let reader = || -> Result<BufReader<File>> { Ok(BufReader::new(File::open(path)?)) };
    match ext.as_str() {
        "gif" => {
            let d = image::codecs::gif::GifDecoder::new(reader()?).map_err(err)?;
            collect(d.into_frames().collect_frames().map_err(err)?)
        }
        "webp" => {
            let d = image::codecs::webp::WebPDecoder::new(reader()?).map_err(err)?;
            collect(d.into_frames().collect_frames().map_err(err)?)
        }
        "png" | "apng" => {
            let d = image::codecs::png::PngDecoder::new(reader()?).map_err(err)?;
            if d.is_apng().map_err(err)? {
                collect(d.apng().map_err(err)?.into_frames().collect_frames().map_err(err)?)
            } else {
                let f = load_image(path)?;
                Ok(Animation { width: f.width, height: f.height, frames: vec![Arc::new(f)], delays: vec![1.0] })
            }
        }
        _ => {
            let f = load_image(path)?;
            Ok(Animation { width: f.width, height: f.height, frames: vec![Arc::new(f)], delays: vec![1.0] })
        }
    }
}

pub fn is_animated_webp(path: &Path) -> bool {
    File::open(path)
        .ok()
        .and_then(|f| image::codecs::webp::WebPDecoder::new(BufReader::new(f)).ok())
        .map(|d| d.has_animation())
        .unwrap_or(false)
}

fn svg_tree(path: &Path) -> Result<resvg::usvg::Tree> {
    let data = std::fs::read(path)?;
    let mut opt = resvg::usvg::Options { resources_dir: path.parent().map(|p| p.to_path_buf()), ..Default::default() };
    opt.fontdb_mut().load_system_fonts();
    resvg::usvg::Tree::from_data(&data, &opt).map_err(|e| MediaError::Decode(format!("{}: {e}", path.display())))
}

pub fn svg_size(path: &Path) -> Result<(u32, u32)> {
    let t = svg_tree(path)?;
    let s = t.size();
    Ok((s.width().ceil() as u32, s.height().ceil() as u32))
}

/// Rasterize an SVG at an exact pixel size (vector-sharp at any scale). Output is premultiplied.
pub fn render_svg(path: &Path, width: u32, height: u32) -> Result<Frame> {
    let tree = svg_tree(path)?;
    let size = tree.size();
    let w = width.max(1);
    let h = height.max(1);
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or_else(|| MediaError::Decode("svg size".into()))?;
    let tf = resvg::tiny_skia::Transform::from_scale(w as f32 / size.width(), h as f32 / size.height());
    resvg::render(&tree, tf, &mut pixmap.as_mut());
    Ok(Frame::new(w, h, pixmap.take(), true))
}
