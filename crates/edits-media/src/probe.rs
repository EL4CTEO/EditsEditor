//! Asset kind detection and metadata probing.

use std::path::Path;

use edits_core::{AssetKind, MediaInfo};
use serde_json::Value;

use crate::{MediaError, Result, ffmpeg::Ffmpeg, image_io, sequence::ImageSequence};

pub const IMAGE_EXT: &[&str] =
    &["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "tga", "ico", "hdr", "exr", "qoi", "pnm", "ppm", "pgm", "dds", "avif"];
pub const VIDEO_EXT: &[&str] =
    &["mp4", "mkv", "mov", "avi", "webm", "m4v", "flv", "wmv", "ts", "m2ts", "mts", "mpg", "mpeg", "3gp", "ogv", "mxf", "vob"];
pub const AUDIO_EXT: &[&str] = &["mp3", "wav", "flac", "ogg", "opus", "m4a", "aac", "wma", "aiff", "aif", "alac", "mka"];
pub const FONT_EXT: &[&str] = &["ttf", "otf", "ttc", "otc"];
pub const SUB_EXT: &[&str] = &["lrc", "srt", "vtt", "ass", "ssa"];
pub const DATA_EXT: &[&str] = &["json", "csv", "txt", "tsv", "md"];

fn ext_of(path: &str) -> String {
    Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// Guess an asset kind from the path alone.
pub fn guess_kind(path: &str) -> AssetKind {
    if path.contains('%') || path.contains("####") || Path::new(path).is_dir() {
        return AssetKind::ImageSequence;
    }
    let e = ext_of(path);
    let e = e.as_str();
    if e == "svg" || e == "svgz" {
        AssetKind::Svg
    } else if e == "gif" || e == "apng" {
        AssetKind::AnimatedImage
    } else if IMAGE_EXT.contains(&e) {
        AssetKind::Image
    } else if VIDEO_EXT.contains(&e) {
        AssetKind::Video
    } else if AUDIO_EXT.contains(&e) {
        AssetKind::Audio
    } else if FONT_EXT.contains(&e) {
        AssetKind::Font
    } else if e == "cube" {
        AssetKind::Lut
    } else if SUB_EXT.contains(&e) {
        AssetKind::Subtitles
    } else if DATA_EXT.contains(&e) {
        AssetKind::Data
    } else {
        AssetKind::Auto
    }
}

fn parse_rate(s: &str) -> Option<f64> {
    let (a, b) = s.split_once('/').unwrap_or((s, "1"));
    let a: f64 = a.parse().ok()?;
    let b: f64 = b.parse().ok()?;
    if a <= 0.0 || b <= 0.0 { None } else { Some(a / b) }
}

/// Probe a file with ffprobe.
pub fn ffprobe(ff: &Ffmpeg, path: &Path) -> Result<(MediaInfo, Value)> {
    let out = ff.probe_cmd().args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"]).arg(path).output()?;
    if !out.status.success() {
        return Err(MediaError::Probe(format!("{}: {}", path.display(), String::from_utf8_lossy(&out.stderr).trim())));
    }
    let v: Value = serde_json::from_slice(&out.stdout).map_err(|e| MediaError::Probe(e.to_string()))?;
    let mut info = MediaInfo::default();
    let streams = v["streams"].as_array().cloned().unwrap_or_default();
    for s in &streams {
        let attached = s["disposition"]["attached_pic"].as_i64() == Some(1);
        match s["codec_type"].as_str() {
            Some("video") if !attached && !info.has_video => {
                info.has_video = true;
                info.width = s["width"].as_u64().unwrap_or(0) as u32;
                info.height = s["height"].as_u64().unwrap_or(0) as u32;
                // rotation metadata (phones): swap dimensions for 90/270
                let rot = s["side_data_list"]
                    .as_array()
                    .and_then(|l| l.iter().find_map(|d| d["rotation"].as_f64()))
                    .or_else(|| s["tags"]["rotate"].as_str().and_then(|r| r.parse().ok()))
                    .unwrap_or(0.0);
                if (rot.abs() - 90.0).abs() < 1.0 || (rot.abs() - 270.0).abs() < 1.0 {
                    std::mem::swap(&mut info.width, &mut info.height);
                }
                info.fps = s["avg_frame_rate"].as_str().and_then(parse_rate).or_else(|| s["r_frame_rate"].as_str().and_then(parse_rate));
                info.video_codec = s["codec_name"].as_str().map(String::from);
                info.frames = s["nb_frames"].as_str().and_then(|n| n.parse().ok());
                let pix = s["pix_fmt"].as_str().unwrap_or("");
                info.has_alpha = pix.contains('a')
                    && (pix.starts_with("yuva")
                        || pix.starts_with("rgba")
                        || pix.starts_with("argb")
                        || pix.starts_with("bgra")
                        || pix.starts_with("gbrap")
                        || pix == "pal8");
            }
            Some("audio") if !info.has_audio => {
                info.has_audio = true;
                info.audio_codec = s["codec_name"].as_str().map(String::from);
                info.sample_rate = s["sample_rate"].as_str().and_then(|r| r.parse().ok());
                info.channels = s["channels"].as_u64().map(|c| c as u32);
            }
            _ => {}
        }
    }
    info.duration = v["format"]["duration"]
        .as_str()
        .and_then(|d| d.parse().ok())
        .or_else(|| streams.iter().find_map(|s| s["duration"].as_str().and_then(|d| d.parse().ok())));
    Ok((info, v))
}

/// Detect the kind (if `Auto`) and gather metadata for an asset path.
pub fn probe_asset(ff: Option<&Ffmpeg>, path: &Path, kind: AssetKind) -> Result<(AssetKind, MediaInfo)> {
    let p = path.to_string_lossy().to_string();
    let mut kind = if kind == AssetKind::Auto { guess_kind(&p) } else { kind };
    match kind {
        AssetKind::Image => {
            let (w, h) = image::image_dimensions(path).map_err(|e| MediaError::Decode(format!("{}: {e}", path.display())))?;
            // webp can be animated
            if ext_of(&p) == "webp" && image_io::is_animated_webp(path) {
                kind = AssetKind::AnimatedImage;
            } else {
                return Ok((kind, MediaInfo { width: w, height: h, has_video: true, has_alpha: true, ..Default::default() }));
            }
        }
        AssetKind::Svg => {
            let (w, h) = image_io::svg_size(path)?;
            return Ok((kind, MediaInfo { width: w, height: h, has_video: true, has_alpha: true, ..Default::default() }));
        }
        AssetKind::ImageSequence => {
            let seq = ImageSequence::open(&p)?;
            let (w, h) = seq.dimensions()?;
            return Ok((
                kind,
                MediaInfo { width: w, height: h, has_video: true, frames: Some(seq.len() as u64), has_alpha: true, ..Default::default() },
            ));
        }
        AssetKind::Font | AssetKind::Lut | AssetKind::Subtitles | AssetKind::Data => {
            if !path.exists() {
                return Err(MediaError::NotFound(p));
            }
            return Ok((kind, MediaInfo::default()));
        }
        _ => {}
    }
    if kind == AssetKind::AnimatedImage {
        let anim = image_io::load_animation(path)?;
        let dur: f64 = anim.delays.iter().sum();
        return Ok((
            kind,
            MediaInfo {
                width: anim.width,
                height: anim.height,
                has_video: true,
                has_alpha: true,
                duration: Some(dur),
                frames: Some(anim.frames.len() as u64),
                fps: Some(anim.frames.len() as f64 / dur.max(1e-6)),
                ..Default::default()
            },
        ));
    }
    let ff = ff.ok_or(MediaError::FfmpegMissing)?;
    if !path.exists() {
        return Err(MediaError::NotFound(p));
    }
    let (info, _) = ffprobe(ff, path)?;
    if kind == AssetKind::Auto || kind == AssetKind::Video || kind == AssetKind::Audio {
        kind = if info.has_video && info.duration.unwrap_or(0.0) > 0.0 && info.frames != Some(1) {
            AssetKind::Video
        } else if info.has_video {
            AssetKind::Image
        } else if info.has_audio {
            AssetKind::Audio
        } else {
            return Err(MediaError::Probe(format!("{}: no audio or video streams", path.display())));
        };
    }
    Ok((kind, info))
}
