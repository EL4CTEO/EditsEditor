//! Encoding rendered frames through an FFmpeg pipe, with hardware encoder auto-selection
//! (NVENC / AMF / QSV on Windows) and many output formats.

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Stdio},
    thread::JoinHandle,
};

use crossbeam_channel::{Sender, bounded};
use serde::{Deserialize, Serialize};

use crate::{MediaError, Result, ffmpeg::Ffmpeg};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Codec {
    /// Pick from the file extension (.mp4 → h264, .mov → prores, .webm → vp9, .gif, .png ...).
    #[default]
    Auto,
    H264,
    H265,
    Av1,
    /// ProRes 422 HQ (.mov) — editing-friendly master.
    Prores,
    /// ProRes 4444 with alpha (.mov).
    Prores4444,
    /// VP9 with alpha (.webm).
    Vp9,
    Gif,
    /// Animated WebP.
    Webp,
    /// Animated PNG.
    Apng,
    /// PNG image sequence (path should contain %05d or a directory is created).
    PngSequence,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ExportSettings {
    /// Output file path.
    pub output: String,
    #[serde(default)]
    pub codec: Codec,
    /// 0..100 (higher = better). Presets: draft 40, good 70, high 85, max 100.
    #[serde(default = "default_quality")]
    pub quality: f64,
    /// Use a hardware encoder (NVENC/AMF/QSV) when available.
    #[serde(default = "yes")]
    pub hardware: bool,
    /// Output size override (scales the render).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    /// Include audio.
    #[serde(default = "yes")]
    pub audio: bool,
    /// Audio bitrate in kbps (lossy codecs).
    #[serde(default = "default_abr")]
    pub audio_bitrate: u32,
    /// Export only this time range [start, end] (seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<[f64; 2]>,
    /// Extra raw FFmpeg output arguments (advanced).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_args: Vec<String>,
}

fn default_quality() -> f64 {
    85.0
}
fn yes() -> bool {
    true
}
fn default_abr() -> u32 {
    320
}

impl ExportSettings {
    pub fn new(output: impl Into<String>) -> Self {
        ExportSettings {
            output: output.into(),
            codec: Codec::Auto,
            quality: default_quality(),
            hardware: true,
            width: None,
            height: None,
            audio: true,
            audio_bitrate: default_abr(),
            range: None,
            extra_args: vec![],
        }
    }

    pub fn resolved_codec(&self) -> Codec {
        if self.codec != Codec::Auto {
            return self.codec;
        }
        let ext = Path::new(&self.output).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        match ext.as_str() {
            "mov" => Codec::Prores,
            "webm" => Codec::Vp9,
            "gif" => Codec::Gif,
            "webp" => Codec::Webp,
            "apng" => Codec::Apng,
            "png" => Codec::PngSequence,
            "mkv" => Codec::H265,
            _ => Codec::H264,
        }
    }

    /// Does the output keep transparency?
    pub fn has_alpha(&self) -> bool {
        matches!(self.resolved_codec(), Codec::Prores4444 | Codec::Vp9 | Codec::Webp | Codec::Apng | Codec::PngSequence | Codec::Gif)
    }
}

/// "24000/1001" style rate strings for NTSC rates.
pub fn fps_rational(fps: f64) -> String {
    for (n, d) in [(24000, 1001), (30000, 1001), (60000, 1001), (120000, 1001)] {
        if (fps - n as f64 / d as f64).abs() < 0.002 {
            return format!("{n}/{d}");
        }
    }
    if (fps - fps.round()).abs() < 1e-6 { format!("{}", fps.round() as u64) } else { format!("{}/1000", (fps * 1000.0).round() as u64) }
}

fn crf_from_quality(q: f64, lo: f64, hi: f64) -> String {
    // q=100 -> lo (best), q=0 -> hi (worst)
    format!("{:.0}", hi - (q.clamp(0.0, 100.0) / 100.0) * (hi - lo))
}

/// The FFmpeg video encoder arguments chosen for these settings (also reported to agents).
pub fn video_args(ff: &Ffmpeg, s: &ExportSettings) -> (Vec<String>, String) {
    let q = s.quality;
    let hw = s.hardware;
    let pick = |names: &[&str]| -> Option<String> { names.iter().find(|n| ff.encoder_works(n)).map(|n| n.to_string()) };
    let mut a: Vec<String> = vec![];
    let push = |a: &mut Vec<String>, xs: &[&str]| a.extend(xs.iter().map(|x| x.to_string()));
    let codec = s.resolved_codec();
    let name;
    match codec {
        Codec::H264 | Codec::Auto | Codec::H265 | Codec::Av1 => {
            let (hw_list, sw): (&[&str], &[&str]) = match codec {
                Codec::H265 => (&["hevc_nvenc", "hevc_amf", "hevc_qsv"], &["libx265"]),
                Codec::Av1 => (&["av1_nvenc", "av1_amf", "av1_qsv"], &["libsvtav1", "libaom-av1"]),
                _ => (&["h264_nvenc", "h264_amf", "h264_qsv"], &["libx264"]),
            };
            let enc = if hw { pick(hw_list) } else { None }.or_else(|| pick(sw)).unwrap_or_else(|| "mpeg4".into());
            name = enc.clone();
            push(&mut a, &["-c:v", &enc]);
            if enc.ends_with("_nvenc") {
                push(&mut a, &["-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq", &crf_from_quality(q, 14.0, 40.0), "-b:v", "0"]);
            } else if enc.ends_with("_amf") {
                let qp = crf_from_quality(q, 14.0, 40.0);
                push(&mut a, &["-quality", "quality", "-rc", "cqp", "-qp_i", &qp, "-qp_p", &qp, "-qp_b", &qp]);
            } else if enc.ends_with("_qsv") {
                push(&mut a, &["-preset", "slow", "-global_quality", &crf_from_quality(q, 14.0, 40.0)]);
            } else if enc == "libx264" {
                push(&mut a, &["-preset", "medium", "-crf", &crf_from_quality(q, 12.0, 35.0)]);
            } else if enc == "libx265" {
                push(&mut a, &["-preset", "medium", "-crf", &crf_from_quality(q, 14.0, 38.0), "-tag:v", "hvc1"]);
            } else if enc == "libsvtav1" {
                push(&mut a, &["-preset", "7", "-crf", &crf_from_quality(q, 18.0, 50.0)]);
            } else if enc == "libaom-av1" {
                push(&mut a, &["-cpu-used", "6", "-row-mt", "1", "-crf", &crf_from_quality(q, 18.0, 50.0), "-b:v", "0"]);
            } else {
                push(&mut a, &["-q:v", "3"]);
            }
            push(&mut a, &["-pix_fmt", "yuv420p"]);
        }
        Codec::Prores => {
            name = "prores_ks".into();
            push(&mut a, &["-c:v", "prores_ks", "-profile:v", "3", "-pix_fmt", "yuv422p10le", "-vendor", "apl0"]);
        }
        Codec::Prores4444 => {
            name = "prores_ks".into();
            push(&mut a, &["-c:v", "prores_ks", "-profile:v", "4", "-pix_fmt", "yuva444p10le", "-alpha_bits", "16", "-vendor", "apl0"]);
        }
        Codec::Vp9 => {
            name = "libvpx-vp9".into();
            push(
                &mut a,
                &[
                    "-c:v",
                    "libvpx-vp9",
                    "-crf",
                    &crf_from_quality(q, 12.0, 45.0),
                    "-b:v",
                    "0",
                    "-row-mt",
                    "1",
                    "-deadline",
                    "good",
                    "-cpu-used",
                    "2",
                    "-pix_fmt",
                    "yuva420p",
                ],
            );
        }
        Codec::Gif => {
            name = "gif".into();
            push(
                &mut a,
                &[
                    "-filter_complex",
                    "[0:v]split[a][b];[a]palettegen=stats_mode=diff:reserve_transparent=1[p];[b][p]paletteuse=dither=sierra2_4a:alpha_threshold=128",
                    "-loop",
                    "0",
                ],
            );
        }
        Codec::Webp => {
            let enc = pick(&["libwebp_anim", "libwebp"]).unwrap_or_else(|| "libwebp".into());
            name = enc.clone();
            push(
                &mut a,
                &["-c:v", &enc, "-lossless", "0", "-q:v", &format!("{:.0}", q.clamp(1.0, 100.0)), "-loop", "0", "-pix_fmt", "yuva420p"],
            );
        }
        Codec::Apng => {
            name = "apng".into();
            push(&mut a, &["-c:v", "apng", "-plays", "0", "-f", "apng"]);
        }
        Codec::PngSequence => {
            name = "png".into();
            push(&mut a, &["-c:v", "png", "-pix_fmt", "rgba"]);
        }
    }
    (a, name)
}

fn audio_args(s: &ExportSettings) -> Vec<String> {
    let br = format!("{}k", s.audio_bitrate);
    match s.resolved_codec() {
        Codec::Prores | Codec::Prores4444 => vec!["-c:a".into(), "pcm_s16le".into()],
        Codec::Vp9 => vec!["-c:a".into(), "libopus".into(), "-b:a".into(), br],
        Codec::Gif | Codec::Webp | Codec::Apng | Codec::PngSequence => vec!["-an".into()],
        _ => vec!["-c:a".into(), "aac".into(), "-b:a".into(), br],
    }
}

pub struct EncoderConfig<'a> {
    pub settings: &'a ExportSettings,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    /// Pre-mixed audio WAV to mux (if any).
    pub audio_wav: Option<PathBuf>,
}

pub struct Encoder {
    child: Child,
    tx: Option<Sender<Vec<u8>>>,
    writer: Option<JoinHandle<std::io::Result<()>>>,
    stderr: Option<JoinHandle<String>>,
    frame_bytes: usize,
    pub frames_written: u64,
    pub encoder_name: String,
    pub output: PathBuf,
}

impl Encoder {
    pub fn start(ff: &Ffmpeg, cfg: EncoderConfig) -> Result<Encoder> {
        let s = cfg.settings;
        let mut output = PathBuf::from(&s.output);
        if let Some(parent) = output.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let codec = s.resolved_codec();
        if codec == Codec::PngSequence && !s.output.contains('%') {
            // write into a directory next to the requested name
            let dir = output.with_extension("");
            std::fs::create_dir_all(&dir)?;
            output = dir.join("frame_%05d.png");
        }
        let mut cmd = ff.cmd();
        cmd.arg("-y");
        cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba", "-s", &format!("{}x{}", cfg.width, cfg.height)]);
        cmd.args(["-framerate", &fps_rational(cfg.fps), "-i", "-"]);
        let with_audio =
            s.audio && cfg.audio_wav.is_some() && !matches!(codec, Codec::Gif | Codec::Webp | Codec::Apng | Codec::PngSequence);
        if with_audio {
            cmd.arg("-i").arg(cfg.audio_wav.as_ref().unwrap());
        }
        let (vargs, name) = video_args(ff, s);
        // scale if requested (even dims for yuv420)
        let wants_scale = s.width.is_some() || s.height.is_some();
        if wants_scale && codec != Codec::Gif {
            let w = s.width.map(|w| w as i64).unwrap_or(-2);
            let h = s.height.map(|h| h as i64).unwrap_or(-2);
            cmd.args(["-vf", &format!("scale={w}:{h}:flags=lanczos")]);
        }
        cmd.args(&vargs);
        if with_audio {
            cmd.args(["-map", "0:v:0", "-map", "1:a:0"]);
            cmd.args(audio_args(s));
            cmd.arg("-shortest");
        } else {
            cmd.arg("-an");
        }
        let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        if ext == "mp4" || ext == "mov" || ext == "m4v" {
            cmd.args(["-movflags", "+faststart"]);
        }
        cmd.args(&s.extra_args);
        cmd.arg(&output);
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped());
        tracing::debug!(?cmd, "starting encoder");
        let mut child = cmd.spawn()?;
        let mut stdin = child.stdin.take().unwrap();
        let mut stderr_pipe = child.stderr.take().unwrap();
        let stderr = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr_pipe.read_to_string(&mut s);
            s
        });
        let (tx, rx) = bounded::<Vec<u8>>(4);
        let writer = std::thread::Builder::new()
            .name("edits-encode".into())
            .spawn(move || -> std::io::Result<()> {
                for frame in rx {
                    stdin.write_all(&frame)?;
                }
                stdin.flush()?;
                Ok(())
            })
            .map_err(MediaError::Io)?;
        Ok(Encoder {
            child,
            tx: Some(tx),
            writer: Some(writer),
            stderr: Some(stderr),
            frame_bytes: (cfg.width * cfg.height * 4) as usize,
            frames_written: 0,
            encoder_name: name,
            output,
        })
    }

    /// Queue one RGBA8 (straight alpha, sRGB) frame.
    pub fn push(&mut self, frame: Vec<u8>) -> Result<()> {
        if frame.len() != self.frame_bytes {
            return Err(MediaError::Encode(format!("frame has {} bytes, expected {}", frame.len(), self.frame_bytes)));
        }
        self.tx.as_ref().unwrap().send(frame).map_err(|_| MediaError::Encode("encoder pipe closed (ffmpeg exited early)".into()))?;
        self.frames_written += 1;
        Ok(())
    }

    pub fn finish(mut self) -> Result<PathBuf> {
        drop(self.tx.take());
        let wres = self.writer.take().unwrap().join();
        let status = self.child.wait()?;
        let err = self.stderr.take().unwrap().join().unwrap_or_default();
        if !status.success() {
            return Err(MediaError::Encode(format!("ffmpeg failed ({status}): {}", err.trim())));
        }
        if let Ok(Err(e)) = wres {
            return Err(MediaError::Encode(format!("pipe write failed: {e}; {}", err.trim())));
        }
        Ok(self.output.clone())
    }
}

impl Drop for Encoder {
    fn drop(&mut self) {
        if self.tx.is_some() {
            drop(self.tx.take());
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Encode a pre-mixed WAV into an audio-only file (mp3, wav, flac, aac, ogg, opus).
pub fn encode_audio_file(ff: &Ffmpeg, wav: &Path, output: &Path, bitrate_kbps: u32) -> Result<()> {
    let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mut cmd = ff.cmd();
    cmd.arg("-y").arg("-i").arg(wav);
    match ext.as_str() {
        "wav" => {
            cmd.args(["-c:a", "pcm_s16le"]);
        }
        "flac" => {
            cmd.args(["-c:a", "flac"]);
        }
        "mp3" => {
            cmd.args(["-c:a", "libmp3lame", "-b:a", &format!("{bitrate_kbps}k")]);
        }
        "ogg" | "opus" => {
            cmd.args(["-c:a", "libopus", "-b:a", &format!("{bitrate_kbps}k")]);
        }
        _ => {
            cmd.args(["-c:a", "aac", "-b:a", &format!("{bitrate_kbps}k")]);
        }
    }
    let out = cmd.arg(output).stdout(Stdio::null()).stderr(Stdio::piped()).output()?;
    if !out.status.success() {
        return Err(MediaError::Encode(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(())
}
