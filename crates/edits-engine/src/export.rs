//! Export: GPU rendering pipelined with readback and encoding.
//!
//! Frame N is rendered and its readback submitted, then frame N+1 is recorded while N copies
//! back; the encoder runs on its own thread with a bounded queue and video decoders prefetch
//! on theirs — so GPU, decode and encode all overlap.

use std::{
    cell::RefCell,
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use edits_media::{Encoder, EncoderConfig, ExportSettings};
use edits_render::OutputMode;
use serde::Serialize;

use crate::{EngineError, Result, engine::Engine, eval::EvalCtx};

#[derive(Clone, Debug, Serialize)]
pub struct ExportReport {
    pub output: String,
    pub frames: u64,
    pub duration: f64,
    pub elapsed_seconds: f64,
    pub render_fps: f64,
    pub encoder: String,
    pub audio: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub frame: u64,
    pub total: u64,
    pub elapsed: f64,
    pub eta: f64,
}

impl Engine {
    /// Export the root composition (or `comp`). `progress` returns false to cancel.
    pub fn export(&mut self, comp: Option<&str>, settings: &ExportSettings, cancel: Option<Arc<AtomicBool>>, mut progress: impl FnMut(&Progress)) -> Result<ExportReport> {
        let start = Instant::now();
        let comp_id = comp.map(String::from).unwrap_or_else(|| self.project.root.clone());
        let c = self.project.compositions.get(&comp_id).cloned().ok_or_else(|| EngineError::NotFound(format!("composition '{comp_id}'")))?;
        let (t0, t1) = match settings.range {
            Some([a, b]) => (a.max(0.0), b.min(c.duration).max(a)),
            None => (0.0, c.duration),
        };
        let fps = c.fps;
        let total = ((t1 - t0) * fps).round().max(1.0) as u64;
        let ext = std::path::Path::new(&settings.output).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let audio_only = matches!(ext.as_str(), "mp3" | "wav" | "flac" | "aac" | "m4a" | "ogg" | "opus");
        let ff = self.media.ffmpeg()?.clone();

        // ---- audio ----
        let tmp = tempfile::Builder::new().prefix("edits-mix").suffix(".wav").tempfile()?;
        let wav_path = tmp.path().to_path_buf();
        let with_audio = (settings.audio || audio_only) && self.has_audio(Some(&comp_id));
        if with_audio {
            let mix = self.mix_audio(Some(&comp_id), t0, t1)?;
            mix.write_wav(&wav_path)?;
        }
        if audio_only {
            if !with_audio {
                return Err(EngineError::Invalid("composition has no audio".into()));
            }
            edits_media::encode::encode_audio_file(&ff, &wav_path, std::path::Path::new(&settings.output), settings.audio_bitrate)?;
            return Ok(ExportReport {
                output: settings.output.clone(),
                frames: 0,
                duration: t1 - t0,
                elapsed_seconds: start.elapsed().as_secs_f64(),
                render_fps: 0.0,
                encoder: "audio".into(),
                audio: true,
                warnings: vec![],
            });
        }

        // ---- video ----
        let mut encoder = Encoder::start(
            &ff,
            EncoderConfig { settings, width: c.width, height: c.height, fps, audio_wav: with_audio.then(|| wav_path.clone()) },
        )?;
        let encoder_name = encoder.encoder_name.clone();
        let mode = if settings.has_alpha() { OutputMode::Straight } else { OutputMode::Over([0.0, 0.0, 0.0, 1.0]) };
        self.renderer()?;
        let lib = self.library();
        let timing = self.timing_data();
        let vars = Arc::new(self.project.variables.clone());
        let mut warnings: Vec<String> = vec![];
        {
            let Engine { renderer, media, project, text, expr, .. } = self;
            let renderer = renderer.as_mut().unwrap();
            let mut pending: VecDeque<edits_render::PendingReadback> = VecDeque::new();
            for i in 0..total {
                if cancel.as_ref().is_some_and(|c| c.load(Ordering::Relaxed)) {
                    return Err(EngineError::Cancelled);
                }
                let t = t0 + i as f64 / fps;
                let mut ctx = EvalCtx {
                    project,
                    lib: &lib,
                    media,
                    text,
                    expr,
                    timing: timing.clone(),
                    vars: vars.clone(),
                    warnings: RefCell::new(vec![]),
                    expr_errors: Default::default(),
                    root_time: t,
                };
                let mut f = renderer.frame();
                let out = ctx.render_comp(&mut f, &comp_id, t, 0)?;
                pending.push_back(f.read_async(out, mode)?);
                for w in ctx.warnings.into_inner().into_iter().chain(ctx.expr_errors.borrow().iter().cloned()) {
                    if !warnings.contains(&w) && warnings.len() < 50 {
                        warnings.push(w);
                    }
                }
                if pending.len() >= 2 {
                    let frame = pending.pop_front().unwrap().wait()?;
                    encoder.push(frame.data)?;
                }
                let el = start.elapsed().as_secs_f64();
                progress(&Progress { frame: i + 1, total, elapsed: el, eta: el / (i + 1) as f64 * (total - i - 1) as f64 });
            }
            while let Some(p) = pending.pop_front() {
                encoder.push(p.wait()?.data)?;
            }
        }
        let out = encoder.finish()?;
        let elapsed = start.elapsed().as_secs_f64();
        Ok(ExportReport {
            output: out.display().to_string(),
            frames: total,
            duration: t1 - t0,
            elapsed_seconds: elapsed,
            render_fps: total as f64 / elapsed.max(1e-6),
            encoder: encoder_name,
            audio: with_audio,
            warnings,
        })
    }
}
