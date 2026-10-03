//! Audio playback for the viewer (Windows: WASAPI via cpal). Other platforms are silent.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use edits_media::AudioBuffer;

pub struct AudioPlayer {
    /// Playhead in source frames (48 kHz).
    pos: Arc<AtomicU64>,
    playing: Arc<AtomicBool>,
    buffer: Arc<parking::Swap>,
    #[cfg(windows)]
    _stream: Option<cpal::Stream>,
}

/// Tiny lock-free-ish buffer swap (mutex is only taken when replacing the mix).
mod parking {
    use std::sync::{Arc, Mutex};

    use edits_media::AudioBuffer;

    #[derive(Default)]
    pub struct Swap(pub Mutex<Option<Arc<AudioBuffer>>>);

    impl Swap {
        pub fn get(&self) -> Option<Arc<AudioBuffer>> {
            self.0.lock().ok().and_then(|g| g.clone())
        }
        pub fn set(&self, b: Option<Arc<AudioBuffer>>) {
            if let Ok(mut g) = self.0.lock() {
                *g = b;
            }
        }
    }
}

impl AudioPlayer {
    pub fn new() -> AudioPlayer {
        let pos = Arc::new(AtomicU64::new(0));
        let playing = Arc::new(AtomicBool::new(false));
        let buffer = Arc::new(parking::Swap::default());
        #[cfg(windows)]
        let stream = Self::start(pos.clone(), playing.clone(), buffer.clone());
        AudioPlayer {
            pos,
            playing,
            buffer,
            #[cfg(windows)]
            _stream: stream,
        }
    }

    #[cfg(windows)]
    fn start(pos: Arc<AtomicU64>, playing: Arc<AtomicBool>, buffer: Arc<parking::Swap>) -> Option<cpal::Stream> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let dev = host.default_output_device()?;
        let cfg = dev.default_output_config().ok()?;
        if cfg.sample_format() != cpal::SampleFormat::F32 {
            tracing::warn!("audio device is not f32; viewer audio disabled");
            return None;
        }
        let channels = cfg.channels() as usize;
        let rate = cfg.sample_rate() as f64;
        let step = edits_media::SAMPLE_RATE as f64 / rate;
        let config: cpal::StreamConfig = cfg.into();
        let stream = dev
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let buf = buffer.get();
                    let on = playing.load(Ordering::Relaxed);
                    let mut p = pos.load(Ordering::Relaxed) as f64;
                    for frame in data.chunks_mut(channels) {
                        let (l, r) = match (&buf, on) {
                            (Some(b), true) => b.sample_at(p),
                            _ => (0.0, 0.0),
                        };
                        for (i, s) in frame.iter_mut().enumerate() {
                            *s = if i % 2 == 0 { l } else { r };
                        }
                        if on {
                            p += step;
                        }
                    }
                    if on {
                        pos.store(p as u64, Ordering::Relaxed);
                    }
                },
                |e| tracing::warn!("audio stream error: {e}"),
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(stream)
    }

    pub fn set_mix(&self, mix: Option<Arc<AudioBuffer>>) {
        self.buffer.set(mix);
    }

    pub fn has_mix(&self) -> bool {
        self.buffer.get().is_some()
    }

    pub fn play(&self, t: f64) {
        self.pos.store((t.max(0.0) * edits_media::SAMPLE_RATE as f64) as u64, Ordering::Relaxed);
        self.playing.store(true, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.playing.store(false, Ordering::Relaxed);
    }

    /// Current audio time (drives video sync when audio is available).
    pub fn time(&self) -> f64 {
        self.pos.load(Ordering::Relaxed) as f64 / edits_media::SAMPLE_RATE as f64
    }

    pub fn enabled(&self) -> bool {
        #[cfg(windows)]
        {
            self._stream.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}
