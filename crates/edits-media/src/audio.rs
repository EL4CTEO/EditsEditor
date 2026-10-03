//! Audio decoding to interleaved stereo f32 via FFmpeg.

use std::{path::Path, process::Stdio};

use crate::{MediaError, Result, ffmpeg::Ffmpeg};

pub const SAMPLE_RATE: u32 = 48_000;

/// Interleaved stereo f32 PCM.
#[derive(Clone, Debug)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn frames(&self) -> usize {
        self.samples.len() / self.channels.max(1) as usize
    }

    pub fn duration(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    pub fn silence(seconds: f64) -> AudioBuffer {
        let n = (seconds * SAMPLE_RATE as f64).ceil() as usize;
        AudioBuffer { sample_rate: SAMPLE_RATE, channels: 2, samples: vec![0.0; n * 2] }
    }

    /// Mono downmix.
    pub fn mono(&self) -> Vec<f32> {
        let c = self.channels.max(1) as usize;
        self.samples.chunks_exact(c).map(|f| f.iter().sum::<f32>() / c as f32).collect()
    }

    /// Sample (with linear interpolation) at a fractional frame position; returns (l, r).
    #[inline]
    pub fn sample_at(&self, pos: f64) -> (f32, f32) {
        let n = self.frames();
        if pos < 0.0 || n == 0 {
            return (0.0, 0.0);
        }
        let i = pos.floor() as usize;
        if i + 1 >= n {
            if i < n {
                return (self.samples[i * 2], self.samples[i * 2 + 1]);
            }
            return (0.0, 0.0);
        }
        let f = (pos - i as f64) as f32;
        let a = i * 2;
        let b = a + 2;
        (self.samples[a] + (self.samples[b] - self.samples[a]) * f, self.samples[a + 1] + (self.samples[b + 1] - self.samples[a + 1]) * f)
    }

    /// Write a 32-bit float WAV file.
    pub fn write_wav(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let data_len = (self.samples.len() * 4) as u32;
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        let ch = self.channels as u32;
        f.write_all(b"RIFF")?;
        f.write_all(&(36 + data_len).to_le_bytes())?;
        f.write_all(b"WAVEfmt ")?;
        f.write_all(&16u32.to_le_bytes())?;
        f.write_all(&3u16.to_le_bytes())?; // IEEE float
        f.write_all(&(ch as u16).to_le_bytes())?;
        f.write_all(&self.sample_rate.to_le_bytes())?;
        f.write_all(&(self.sample_rate * ch * 4).to_le_bytes())?;
        f.write_all(&((ch * 4) as u16).to_le_bytes())?;
        f.write_all(&32u16.to_le_bytes())?;
        f.write_all(b"data")?;
        f.write_all(&data_len.to_le_bytes())?;
        let mut bytes = Vec::with_capacity(self.samples.len() * 4);
        for s in &self.samples {
            bytes.extend_from_slice(&s.to_le_bytes());
        }
        f.write_all(&bytes)?;
        f.flush()?;
        Ok(())
    }
}

/// Decode the first audio stream of a file to 48 kHz stereo f32.
pub fn decode_audio(ff: &Ffmpeg, path: &Path) -> Result<AudioBuffer> {
    let out = ff
        .cmd()
        .arg("-i")
        .arg(path)
        .args(["-vn", "-sn", "-dn", "-ac", "2", "-ar", &SAMPLE_RATE.to_string(), "-f", "f32le", "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !out.status.success() && out.stdout.is_empty() {
        return Err(MediaError::Decode(format!(
            "audio decode failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let samples: Vec<f32> = out.stdout.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
    Ok(AudioBuffer { sample_rate: SAMPLE_RATE, channels: 2, samples })
}
