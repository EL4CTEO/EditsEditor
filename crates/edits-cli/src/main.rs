//! `edits` — the EditsEditor command line.

use std::{
    io::{IsTerminal, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use edits_core::{Project, summary, validate::Severity};
use edits_engine::{AnalysisOptions, Engine, ExportSettings, ImportOptions, OutputMode, SceneOptions};
use edits_fx::{EffectKind, Library, PresetScope};
use edits_media::Codec;

#[derive(Parser)]
#[command(name = "edits", version, about = "EditsEditor — GPU video editor for AI agents (AMVs, anime edits, MVs)", long_about = None)]
struct Cli {
    /// Use the software GPU fallback (WARP / lavapipe).
    #[arg(long, global = true, env = "EDITS_SOFTWARE_GPU")]
    software_gpu: bool,
    /// Verbose logging (to stderr).
    #[arg(short, long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Run the MCP server on stdio (point your agent's MCP config at this).
    Mcp {
        /// Project to open at startup.
        #[arg(long, env = "EDITS_PROJECT")]
        project: Option<PathBuf>,
    },
    /// Print an MCP client config snippet for this executable.
    McpConfig,
    /// Create a new project file.
    New {
        path: PathBuf,
        #[arg(long, default_value_t = 1920)]
        width: u32,
        #[arg(long, default_value_t = 1080)]
        height: u32,
        #[arg(long, default_value_t = 24.0)]
        fps: f64,
        #[arg(long, default_value_t = 30.0)]
        duration: f64,
        #[arg(long)]
        name: Option<String>,
    },
    /// Import media files / folders / globs into a project.
    Import {
        project: PathBuf,
        paths: Vec<String>,
        #[arg(long)]
        recursive: bool,
    },
    /// Analyze a music asset (beats, drops, sections) and write it to the project timing.
    Analyze {
        project: PathBuf,
        asset: String,
        #[arg(long)]
        no_apply: bool,
        #[arg(long)]
        bpm: Option<f64>,
    },
    /// Detect shots in a video asset.
    Scenes { project: PathBuf, asset: String },
    /// Print the project summary.
    Summary { project: PathBuf },
    /// Validate the project.
    Validate { project: PathBuf },
    /// Render one frame to an image.
    Frame {
        project: PathBuf,
        #[arg(short, long)]
        time: f64,
        #[arg(short, long, default_value = "frame.png")]
        output: PathBuf,
        /// Keep transparency.
        #[arg(long)]
        alpha: bool,
    },
    /// Render a contact sheet of evenly spaced frames.
    Sheet {
        project: PathBuf,
        #[arg(short, long, default_value = "sheet.jpg")]
        output: PathBuf,
        #[arg(long, default_value_t = 12)]
        count: u32,
        #[arg(long, default_value_t = 4)]
        columns: u32,
    },
    /// Export the edit (video, gif, image sequence or audio).
    Render {
        project: PathBuf,
        #[arg(short, long)]
        output: String,
        #[arg(long, value_enum)]
        codec: Option<CodecArg>,
        #[arg(long, default_value_t = 85.0)]
        quality: f64,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
        #[arg(long, num_args = 2, value_names = ["START", "END"])]
        range: Option<Vec<f64>>,
        /// Disable hardware encoders.
        #[arg(long)]
        no_hw: bool,
        #[arg(long)]
        no_audio: bool,
    },
    /// Apply a preset (clip presets need --target).
    Preset {
        project: PathBuf,
        preset: String,
        #[arg(long)]
        target: Option<String>,
        /// Preset args as JSON.
        #[arg(long)]
        args: Option<String>,
    },
    /// Run a Rhai script file ("-" = stdin) against the project.
    Script {
        project: PathBuf,
        file: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// List effects.
    Effects {
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        query: Option<String>,
    },
    /// List presets.
    Presets {
        #[arg(long)]
        query: Option<String>,
    },
    /// Show documentation (topics: workflow, format, expressions, scripting, effects, presets, easings, blend_modes, tips).
    Doc { topic: Option<String> },
    /// Print the JSON schema of the project file (or a sub-type).
    Schema { r#type: Option<String> },
    /// Generate Markdown catalogs of all effects and presets.
    Catalog {
        #[arg(short, long, default_value = "docs")]
        output: PathBuf,
    },
    /// Render every built-in effect onto a test card into labeled contact sheets.
    Gallery {
        #[arg(short, long, default_value = "gallery")]
        output: PathBuf,
        #[arg(long, default_value_t = 320)]
        width: u32,
    },
    /// Open the live viewer (watches the project file as agents edit it).
    View { project: Option<PathBuf> },
    /// Check GPU, FFmpeg, encoders and fonts.
    Doctor,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum CodecArg {
    H264,
    H265,
    Av1,
    Prores,
    Prores4444,
    Vp9,
    Gif,
    Webp,
    Apng,
    PngSequence,
}

fn open(path: &Path, software: bool) -> Result<Engine> {
    let mut e = Engine::open(path)?;
    e.gpu_options.software = software;
    Ok(e)
}

fn progress_bar(p: &edits_engine::Progress) {
    if !std::io::stderr().is_terminal() && p.frame % 24 != 0 && p.frame != p.total {
        return;
    }
    let w = 30;
    let filled = (p.frame as f64 / p.total as f64 * w as f64) as usize;
    eprint!(
        "\r[{}{}] {}/{} frames  {:.1}s elapsed  eta {:.1}s ",
        "#".repeat(filled),
        "-".repeat(w - filled),
        p.frame,
        p.total,
        p.elapsed,
        p.eta
    );
    let _ = std::io::stderr().flush();
}

fn catalog(out: &Path) -> Result<()> {
    std::fs::create_dir_all(out)?;
    let lib = Library::builtin();
    let mut md = String::from("# Effect catalog\n\nGenerated by `edits catalog`. Every effect is a WGSL shader in `crates/edits-fx/effects/`.\n");
    for kind in [EffectKind::Filter, EffectKind::Transition, EffectKind::Generator] {
        md.push_str(&format!("\n## {}s\n", match kind {
            EffectKind::Filter => "Filter",
            EffectKind::Transition => "Transition",
            EffectKind::Generator => "Generator",
        }));
        let mut list: Vec<_> = lib.effects().filter(|e| e.kind == kind).collect();
        list.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));
        let mut cat = String::new();
        for e in list {
            if e.category != cat {
                md.push_str(&format!("\n### {}\n\n| id | description | params |\n|---|---|---|\n", e.category));
                cat = e.category.clone();
            }
            let params: Vec<String> = e
                .params
                .iter()
                .map(|p| format!("`{}`={}", p.name, serde_json::to_string(&p.default).unwrap_or_default()))
                .collect();
            md.push_str(&format!("| `{}` | {} | {} |\n", e.id, e.description.replace('|', "/"), params.join(" ")));
        }
    }
    std::fs::write(out.join("effects.md"), md)?;
    let mut md = String::from("# Preset catalog\n\nGenerated by `edits catalog`. Presets are Rhai scripts in `crates/edits-fx/presets/`.\n");
    let mut list: Vec<_> = lib.presets().collect();
    list.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));
    let mut cat = String::new();
    for p in list {
        if p.category != cat {
            md.push_str(&format!("\n## {}\n\n| id | scope | description | params |\n|---|---|---|---|\n", p.category));
            cat = p.category.clone();
        }
        let params: Vec<String> = p.params.iter().map(|x| format!("`{}`={}", x.name, serde_json::to_string(&x.default).unwrap_or_default())).collect();
        md.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            p.id,
            if p.scope == PresetScope::Clip { "clip" } else { "timeline" },
            p.description.replace('|', "/"),
            params.join(" ")
        ));
    }
    std::fs::write(out.join("presets.md"), md)?;
    for (name, t) in [
        ("agent-guide.md", edits_mcp::reference::INSTRUCTIONS),
        ("format.md", edits_mcp::reference::FORMAT),
        ("scripting.md", edits_mcp::reference::SCRIPTING),
        ("effect-authoring.md", edits_mcp::reference::EFFECT_AUTHORING),
        ("expressions.md", edits_engine::expr::EXPR_REFERENCE),
        ("tips.md", edits_mcp::reference::TIPS),
    ] {
        std::fs::write(out.join(name), format!("```text\n{t}\n```\n"))?;
    }
    std::fs::write(out.join("project.schema.json"), serde_json::to_string_pretty(&edits_core::project_schema())?)?;
    println!("wrote catalogs to {}", out.display());
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let filter = if cli.verbose { "debug,wgpu_core=warn,wgpu_hal=warn,naga=warn" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .with_writer(std::io::stderr)
        .init();
    let sw = cli.software_gpu;
    match cli.cmd {
        Cmd::Mcp { project } => {
            let gpu = edits_render::GpuOptions { software: sw, ..Default::default() };
            let engine = match project {
                Some(p) if p.exists() => {
                    let mut e = Engine::open(&p)?;
                    e.gpu_options = gpu.clone();
                    Some(e)
                }
                _ => None,
            };
            let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
            rt.block_on(edits_mcp::serve_stdio(engine, gpu))?;
        }
        Cmd::McpConfig => {
            let exe = std::env::current_exe()?;
            let cfg = serde_json::json!({ "mcpServers": { "edits-editor": { "command": exe.display().to_string(), "args": ["mcp"] } } });
            println!("{}", serde_json::to_string_pretty(&cfg)?);
            eprintln!("\nClaude Code: claude mcp add edits-editor -- \"{}\" mcp", exe.display());
        }
        Cmd::New { path, width, height, fps, duration, name } => {
            let name = name.unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("edit").trim_end_matches(".edits").to_string());
            Engine::create(&path, Project::new(&name, width, height, fps, duration))?;
            println!("created {}", path.display());
        }
        Cmd::Import { project, paths, recursive } => {
            let mut e = open(&project, sw)?;
            for a in e.import(&paths, &ImportOptions { recursive, ..Default::default() })? {
                println!("{:<24} {:?} {}", a.id, a.kind, a.path);
            }
        }
        Cmd::Analyze { project, asset, no_apply, bpm } => {
            let mut e = open(&project, sw)?;
            let an = e.analyze_audio(&asset, &AnalysisOptions { bpm_hint: bpm, ..Default::default() }, !no_apply, None)?;
            println!("bpm {:.2} (confidence {:.2}), {} beats, {} downbeats, drops {:?}", an.bpm, an.tempo_confidence, an.beats.len(), an.downbeats.len(), an.drops);
            for s in &an.sections {
                println!("  {:>7.2}-{:<7.2} {:<7} energy {:.2}", s.start, s.end, s.label, s.energy);
            }
        }
        Cmd::Scenes { project, asset } => {
            let mut e = open(&project, sw)?;
            for s in e.detect_scenes(&asset, &SceneOptions::default())?.iter() {
                println!("#{:<3} {:>8.2}-{:<8.2} motion {:.2} bright {:.2} {}", s.index, s.start, s.end, s.motion, s.brightness, s.color);
            }
        }
        Cmd::Summary { project } => {
            let e = open(&project, sw)?;
            print!("{}", summary::summarize(&e.project));
        }
        Cmd::Validate { project } => {
            let mut e = open(&project, sw)?;
            let issues = e.validate();
            for i in &issues {
                println!("[{:?}] {}{}", i.severity, i.object.as_ref().map(|o| format!("{o}: ")).unwrap_or_default(), i.message);
            }
            if issues.iter().any(|i| i.severity == Severity::Error) {
                std::process::exit(1);
            }
            if issues.is_empty() {
                println!("ok");
            }
        }
        Cmd::Frame { project, time, output, alpha } => {
            let mut e = open(&project, sw)?;
            let r = e.render_frame(None, time, if alpha { OutputMode::Straight } else { OutputMode::Over([0.0, 0.0, 0.0, 1.0]) })?;
            for w in &r.warnings {
                eprintln!("warning: {w}");
            }
            let ext = output.extension().and_then(|x| x.to_str()).unwrap_or("png").to_ascii_lowercase();
            let bytes = if ext == "jpg" || ext == "jpeg" { r.frame.to_jpeg(92)? } else { r.frame.to_png()? };
            std::fs::write(&output, bytes)?;
            println!("{} ({:.0} ms)", output.display(), r.ms);
        }
        Cmd::Sheet { project, output, count, columns } => {
            let mut e = open(&project, sw)?;
            let d = e.project.root_comp().map(|c| c.duration).unwrap_or(1.0);
            let times: Vec<f64> = (0..count).map(|i| d * (i as f64 + 0.5) / count as f64).collect();
            let (sheet, warnings) = e.contact_sheet(None, &times, columns, 480)?;
            for w in warnings {
                eprintln!("warning: {w}");
            }
            std::fs::write(&output, sheet.to_jpeg(90)?)?;
            println!("{}", output.display());
        }
        Cmd::Render { project, output, codec, quality, width, height, range, no_hw, no_audio } => {
            let mut e = open(&project, sw)?;
            let mut s = ExportSettings::new(output);
            s.codec = match codec {
                None => Codec::Auto,
                Some(CodecArg::H264) => Codec::H264,
                Some(CodecArg::H265) => Codec::H265,
                Some(CodecArg::Av1) => Codec::Av1,
                Some(CodecArg::Prores) => Codec::Prores,
                Some(CodecArg::Prores4444) => Codec::Prores4444,
                Some(CodecArg::Vp9) => Codec::Vp9,
                Some(CodecArg::Gif) => Codec::Gif,
                Some(CodecArg::Webp) => Codec::Webp,
                Some(CodecArg::Apng) => Codec::Apng,
                Some(CodecArg::PngSequence) => Codec::PngSequence,
            };
            s.quality = quality;
            s.width = width;
            s.height = height;
            s.hardware = !no_hw;
            s.audio = !no_audio;
            s.range = range.map(|r| [r[0], r[1]]);
            let rep = e.export(None, &s, None, progress_bar)?;
            eprintln!();
            for w in &rep.warnings {
                eprintln!("warning: {w}");
            }
            println!(
                "{} — {} frames in {:.1}s ({:.1} fps, encoder {}{})",
                rep.output,
                rep.frames,
                rep.elapsed_seconds,
                rep.render_fps,
                rep.encoder,
                if rep.audio { ", with audio" } else { "" }
            );
        }
        Cmd::Preset { project, preset, target, args } => {
            let mut e = open(&project, sw)?;
            let args: serde_json::Map<String, serde_json::Value> = match args {
                Some(a) => serde_json::from_str(&a).context("--args must be a JSON object")?,
                None => Default::default(),
            };
            let rep = e.apply_preset(&preset, target.as_deref(), &args, None)?;
            for l in rep.logs {
                println!("log: {l}");
            }
            println!("changed: {} result: {}", rep.changed, rep.result);
        }
        Cmd::Script { project, file, dry_run } => {
            let mut e = open(&project, sw)?;
            let code = if file == "-" {
                let mut s = String::new();
                std::io::stdin().read_to_string(&mut s)?;
                s
            } else {
                std::fs::read_to_string(&file)?
            };
            let (rep, _) = e.run_script(&code, serde_json::Value::Null, dry_run)?;
            for l in rep.logs {
                println!("{l}");
            }
            println!("changed: {} result: {}", rep.changed, rep.result);
        }
        Cmd::Effects { kind, query } => {
            let k = match kind.as_deref() {
                Some("filter") => Some(EffectKind::Filter),
                Some("transition") => Some(EffectKind::Transition),
                Some("generator") => Some(EffectKind::Generator),
                _ => None,
            };
            let list = Library::builtin().search_effects(k, None, query.as_deref());
            for e in &list {
                println!("{:<24} {:<10} {:<12} {}", e.id, e.kind.as_str(), e.category, e.description);
            }
            println!("{} effects", list.len());
        }
        Cmd::Presets { query } => {
            let list = Library::builtin().search_presets(None, query.as_deref());
            for p in &list {
                println!("{:<24} {:<9} {:<14} {}", p.id, if p.scope == PresetScope::Clip { "clip" } else { "timeline" }, p.category, p.description);
            }
            println!("{} presets", list.len());
        }
        Cmd::Doc { topic } => match edits_mcp::reference::topic(topic.as_deref().unwrap_or("workflow")) {
            Some(t) => println!("{t}"),
            None => println!("topics: {}", edits_mcp::reference::TOPICS.join(", ")),
        },
        Cmd::Schema { r#type } => {
            let s = match r#type {
                Some(t) => edits_core::type_schema(&t).with_context(|| format!("unknown type; available: {}", edits_core::SCHEMA_TYPES.join(", ")))?,
                None => edits_core::project_schema(),
            };
            println!("{}", serde_json::to_string_pretty(&s)?);
        }
        Cmd::Catalog { output } => catalog(&output)?,
        Cmd::Gallery { output, width } => {
            std::fs::create_dir_all(&output)?;
            let h = width * 9 / 16;
            let mut e = Engine::new(Project::new("gallery", width, h, 24.0, 4.0), None);
            e.gpu_options.software = sw;
            let lib = Library::builtin();
            for (kind, name) in [(EffectKind::Filter, "filters"), (EffectKind::Transition, "transitions"), (EffectKind::Generator, "generators")] {
                let mut list: Vec<_> = lib.effects().filter(|d| d.kind == kind).collect();
                list.sort_by(|a, b| a.category.cmp(&b.category).then(a.id.cmp(&b.id)));
                let mut frames = vec![];
                let mut labels = vec![];
                for d in list {
                    frames.push(e.preview_effect(&d.id, &Default::default(), Some(1.3), 0.5)?);
                    labels.push(d.id.clone());
                }
                let sheet = e.tile(&frames, &labels, 8, width);
                let p = output.join(format!("{name}.jpg"));
                std::fs::write(&p, sheet.to_jpeg(88)?)?;
                println!("{} ({} effects)", p.display(), frames.len());
            }
        }
        Cmd::View { project } => {
            let exe = std::env::current_exe()?;
            let viewer = exe.with_file_name(if cfg!(windows) { "edits-viewer.exe" } else { "edits-viewer" });
            let mut cmd = std::process::Command::new(&viewer);
            if let Some(p) = project {
                cmd.arg(p);
            }
            cmd.spawn().with_context(|| format!("could not start {}", viewer.display()))?;
        }
        Cmd::Doctor => {
            println!("EditsEditor {}", env!("CARGO_PKG_VERSION"));
            match edits_media::Ffmpeg::locate() {
                Ok(f) => {
                    println!("ffmpeg: {} ({})", f.version, f.ffmpeg.display());
                    let hw: Vec<&str> = ["h264_nvenc", "hevc_nvenc", "av1_nvenc", "h264_amf", "hevc_amf", "h264_qsv", "hevc_qsv"]
                        .into_iter()
                        .filter(|n| f.encoder_works(n))
                        .collect();
                    println!("hardware encoders: {}", if hw.is_empty() { "none (software x264/x265)".into() } else { hw.join(", ") });
                    println!("hw decode: {:?}", f.preferred_hwaccel());
                }
                Err(e) => println!("ffmpeg: MISSING — {e}"),
            }
            match edits_render::GpuContext::new(&edits_render::GpuOptions { software: sw, ..Default::default() }) {
                Ok(g) => println!("gpu: {}", g.describe()),
                Err(e) => println!("gpu: unavailable — {e}"),
            }
            let lib = Library::builtin();
            println!("library: {} effects, {} presets", lib.effect_count(), lib.preset_count());
            let t = edits_render::TextRenderer::new();
            println!("fonts: {} families", t.families().len());
        }
    }
    Ok(())
}
