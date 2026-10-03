//! # edits-media
//!
//! Everything that touches media files: kind detection and probing, streaming video decode
//! (FFmpeg pipes with prefetch), still/animated images, SVG, image sequences, audio decode,
//! encoding with hardware encoder selection, subtitle/lyrics parsing and shot detection.

pub mod audio;
pub mod encode;
pub mod ffmpeg;
pub mod image_io;
pub mod probe;
pub mod scenes;
pub mod sequence;
pub mod subtitles;
pub mod video;

pub use audio::{AudioBuffer, SAMPLE_RATE, decode_audio};
pub use encode::{Codec, Encoder, EncoderConfig, ExportSettings};
pub use ffmpeg::Ffmpeg;
pub use video::{VideoOptions, VideoReader};

#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("FFmpeg not found. Install it (e.g. `winget install Gyan.FFmpeg`) or set EDITS_FFMPEG to ffmpeg.exe")]
    FfmpegMissing,
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("probe failed: {0}")]
    Probe(String),
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("encode failed: {0}")]
    Encode(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T, E = MediaError> = std::result::Result<T, E>;

/// A decoded RGBA8 image (sRGB). `premultiplied` tells the GPU uploader how to treat alpha.
#[derive(Clone, Debug)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub premultiplied: bool,
}

impl Frame {
    pub fn new(width: u32, height: u32, data: Vec<u8>, premultiplied: bool) -> Frame {
        debug_assert_eq!(data.len(), (width * height * 4) as usize);
        Frame { width, height, data, premultiplied }
    }

    pub fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Frame {
        let mut data = Vec::with_capacity((width * height * 4) as usize);
        for _ in 0..width * height {
            data.extend_from_slice(&rgba);
        }
        Frame::new(width, height, data, false)
    }

    /// Encode as PNG bytes.
    pub fn to_png(&self) -> Result<Vec<u8>> {
        let mut out = std::io::Cursor::new(Vec::new());
        let img = image::RgbaImage::from_raw(self.width, self.height, self.data.clone())
            .ok_or_else(|| MediaError::Encode("bad frame size".into()))?;
        img.write_to(&mut out, image::ImageFormat::Png).map_err(|e| MediaError::Encode(e.to_string()))?;
        Ok(out.into_inner())
    }

    /// Encode as JPEG bytes (alpha is dropped over black).
    pub fn to_jpeg(&self, quality: u8) -> Result<Vec<u8>> {
        let rgb: Vec<u8> = self.data.chunks_exact(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
        let mut out = Vec::new();
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        enc.encode(&rgb, self.width, self.height, image::ExtendedColorType::Rgb8).map_err(|e| MediaError::Encode(e.to_string()))?;
        Ok(out)
    }
}
