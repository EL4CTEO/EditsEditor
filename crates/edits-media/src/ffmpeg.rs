//! Locating and talking to the FFmpeg / FFprobe executables.
//!
//! EditsEditor drives FFmpeg through pipes instead of linking libav*. That keeps the build
//! trivial on Windows (no vcpkg / DLL hell), lets users drop in any FFmpeg build (including
//! ones with NVENC/AMF/QSV), and isolates codec crashes from the editor process.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{MediaError, Result};

#[derive(Clone, Debug)]
pub struct Ffmpeg {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
    pub version: String,
    /// Available encoders (e.g. `libx264`, `h264_nvenc`).
    pub encoders: HashSet<String>,
    /// Available hardware decoders (`cuda`, `d3d11va`, `dxva2`, `qsv`, `vaapi`...).
    pub hwaccels: Vec<String>,
    /// Runtime check results for hardware encoders (compiled-in != hardware present).
    tested: Arc<parking_lot::Mutex<HashMap<String, bool>>>,
}

const EXE: &str = if cfg!(windows) { ".exe" } else { "" };

/// Create a `Command` that never pops up a console window on Windows.
pub fn command(program: &Path) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c.stdin(Stdio::null());
    c
}

fn candidates() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Ok(p) = std::env::var("EDITS_FFMPEG") {
        v.push(PathBuf::from(p));
    }
    if let Ok(d) = std::env::var("EDITS_FFMPEG_DIR") {
        v.push(PathBuf::from(d).join(format!("ffmpeg{EXE}")));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            v.push(dir.join(format!("ffmpeg{EXE}")));
            v.push(dir.join("ffmpeg").join("bin").join(format!("ffmpeg{EXE}")));
            v.push(dir.join("ffmpeg").join(format!("ffmpeg{EXE}")));
        }
    }
    if let Ok(p) = which::which("ffmpeg") {
        v.push(p);
    }
    #[cfg(windows)]
    {
        for base in ["C:\\ffmpeg\\bin", "C:\\Program Files\\ffmpeg\\bin", "C:\\ProgramData\\chocolatey\\bin"] {
            v.push(PathBuf::from(base).join("ffmpeg.exe"));
        }
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            // winget installs: %LOCALAPPDATA%\Microsoft\WinGet\Packages\Gyan.FFmpeg_*\ffmpeg-*\bin\ffmpeg.exe
            let pattern = format!("{local}\\Microsoft\\WinGet\\Packages\\*FFmpeg*\\*\\bin\\ffmpeg.exe");
            if let Ok(paths) = glob::glob(&pattern) {
                v.extend(paths.flatten());
            }
            v.push(PathBuf::from(local).join("Microsoft\\WinGet\\Links\\ffmpeg.exe"));
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            v.push(PathBuf::from(profile).join("scoop\\shims\\ffmpeg.exe"));
        }
    }
    v
}

impl Ffmpeg {
    /// Find FFmpeg: `EDITS_FFMPEG`, `EDITS_FFMPEG_DIR`, next to the executable, PATH, and common
    /// Windows install locations (winget, scoop, chocolatey, C:\ffmpeg).
    pub fn locate() -> Result<Ffmpeg> {
        for c in candidates() {
            if c.is_file() {
                if let Ok(f) = Ffmpeg::from_path(&c) {
                    return Ok(f);
                }
            }
        }
        Err(MediaError::FfmpegMissing)
    }

    pub fn from_path(ffmpeg: &Path) -> Result<Ffmpeg> {
        let dir = ffmpeg.parent().unwrap_or(Path::new("."));
        let mut ffprobe = dir.join(format!("ffprobe{EXE}"));
        if !ffprobe.is_file() {
            ffprobe = which::which("ffprobe").map_err(|_| MediaError::FfmpegMissing)?;
        }
        let out = command(ffmpeg).arg("-version").stdout(Stdio::piped()).stderr(Stdio::null()).output()?;
        if !out.status.success() {
            return Err(MediaError::FfmpegMissing);
        }
        let version = String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").to_string();
        let enc = command(ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?;
        let encoders = String::from_utf8_lossy(&enc.stdout)
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let flags = it.next()?;
                if flags.len() == 6 && (flags.starts_with('V') || flags.starts_with('A')) {
                    it.next().map(String::from)
                } else {
                    None
                }
            })
            .collect();
        let hw = command(ffmpeg)
            .args(["-hide_banner", "-hwaccels"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?;
        let hwaccels = String::from_utf8_lossy(&hw.stdout)
            .lines()
            .skip(1)
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect();
        Ok(Ffmpeg { ffmpeg: ffmpeg.to_path_buf(), ffprobe, version, encoders, hwaccels, tested: Default::default() })
    }

    pub fn has_encoder(&self, name: &str) -> bool {
        self.encoders.contains(name)
    }

    /// Whether an encoder is compiled in *and* actually works on this machine. Hardware encoders
    /// (`*_nvenc`, `*_amf`, `*_qsv`, `*_vaapi`...) are verified once with a tiny trial encode.
    pub fn encoder_works(&self, name: &str) -> bool {
        if !self.has_encoder(name) {
            return false;
        }
        let hw = ["_nvenc", "_amf", "_qsv", "_vaapi", "_videotoolbox", "_mf"].iter().any(|s| name.ends_with(s));
        if !hw {
            return true;
        }
        if let Some(v) = self.tested.lock().get(name) {
            return *v;
        }
        let ok = self
            .cmd()
            .args(["-f", "lavfi", "-i", "color=c=black:s=256x256:r=24:d=0.2", "-frames:v", "3", "-c:v", name, "-f", "null", "-"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        tracing::info!(encoder = name, ok, "hardware encoder probe");
        self.tested.lock().insert(name.to_string(), ok);
        ok
    }

    /// Best hardware decode accelerator for this platform, if any.
    pub fn preferred_hwaccel(&self) -> Option<&'static str> {
        let order: &[&str] = if cfg!(windows) { &["d3d11va", "dxva2", "cuda", "qsv"] } else { &["cuda", "vaapi", "videotoolbox"] };
        order.iter().copied().find(|h| self.hwaccels.iter().any(|x| x == h))
    }

    pub fn cmd(&self) -> Command {
        let mut c = command(&self.ffmpeg);
        c.args(["-hide_banner", "-nostdin", "-loglevel", "error"]);
        c
    }

    pub fn probe_cmd(&self) -> Command {
        command(&self.ffprobe)
    }
}
