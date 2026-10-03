//! The MCP tool surface.

use std::{path::PathBuf, sync::Arc};

use base64::Engine as _;
use edits_core::{EffectInstance, Project, ops, summary, validate::Severity};
use edits_engine::{AnalysisOptions, Engine, ExportSettings, ImportOptions, OutputMode, SceneOptions};
use edits_fx::{EffectDef, EffectKind, PresetDef};
use edits_media::Frame;
use parking_lot::Mutex;
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value as Json, json};

use crate::reference;

pub struct State {
    pub engine: Option<Engine>,
    pub gpu: edits_render::GpuOptions,
}

#[derive(Clone)]
pub struct EditsServer {
    state: Arc<Mutex<State>>,
    tool_router: ToolRouter<Self>,
}

type Res = anyhow::Result<CallToolResult>;

fn text(s: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(s)])
}

fn json_result(v: &impl serde::Serialize) -> CallToolResult {
    text(serde_json::to_string_pretty(v).unwrap_or_default())
}

fn image_block(f: &Frame, max_w: u32, jpeg: bool) -> anyhow::Result<ContentBlock> {
    let frame = if f.width > max_w && max_w > 0 {
        let img = image::RgbaImage::from_raw(f.width, f.height, f.data.clone()).ok_or_else(|| anyhow::anyhow!("bad frame"))?;
        let h = (f.height as f64 * max_w as f64 / f.width as f64).round().max(1.0) as u32;
        let small = image::imageops::resize(&img, max_w, h, image::imageops::FilterType::Triangle);
        Frame::new(max_w, h, small.into_raw(), false)
    } else {
        f.clone()
    };
    let (bytes, mime) = if jpeg { (frame.to_jpeg(88)?, "image/jpeg") } else { (frame.to_png()?, "image/png") };
    Ok(ContentBlock::image(base64::engine::general_purpose::STANDARD.encode(bytes), mime))
}

fn engine(s: &mut State) -> anyhow::Result<&mut Engine> {
    s.engine.as_mut().ok_or_else(|| anyhow::anyhow!("no project open — call project_open first (create:true to make a new one)"))
}

fn issues_text(e: &mut Engine) -> String {
    let issues = e.validate();
    let errs = issues.iter().filter(|i| i.severity == Severity::Error).count();
    let warns = issues.iter().filter(|i| i.severity == Severity::Warning).count();
    if issues.is_empty() {
        "validation: ok".into()
    } else {
        let lines: Vec<String> = issues
            .iter()
            .take(12)
            .map(|i| format!("  [{:?}] {}{}", i.severity, i.object.as_ref().map(|o| format!("{o}: ")).unwrap_or_default(), i.message))
            .collect();
        format!("validation: {errs} errors, {warns} warnings\n{}", lines.join("\n"))
    }
}

// ------------------------------------------------------------------------------------------------
// Parameter types
// ------------------------------------------------------------------------------------------------

#[derive(Deserialize, JsonSchema)]
pub struct OpenArgs {
    /// Project file path (*.edits.json).
    pub path: String,
    /// Create the project if it does not exist.
    #[serde(default)]
    pub create: bool,
    /// For new projects: name, size, fps, duration (seconds).
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub fps: Option<f64>,
    #[serde(default)]
    pub duration: Option<f64>,
}

#[derive(Deserialize, JsonSchema)]
pub struct GetArgs {
    /// Object id (clip, track, effect, asset, composition, marker). Omit for the whole project.
    #[serde(default)]
    pub id: Option<String>,
    /// Dot path inside the object (e.g. "transform.scale", "effects.fx3.params"), or a JSON pointer "/a/b".
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PatchArgs {
    /// Object id to patch; omit to patch the whole project.
    #[serde(default)]
    pub id: Option<String>,
    /// RFC 7396 merge patch (objects merge, null deletes) — the easy option.
    #[serde(default)]
    pub merge: Option<Json>,
    /// RFC 6902 JSON Patch operations (add/remove/replace/move/copy/test) applied to the object JSON.
    #[serde(default)]
    pub ops: Option<Json>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SetArgs {
    /// Object id.
    pub id: String,
    /// Dot path (e.g. "opacity", "transform.position", "effects.fx2.params.amount", "source.text").
    pub path: String,
    /// New value (any JSON). Omit or null together with remove=true to reset to default.
    #[serde(default)]
    pub value: Json,
    #[serde(default)]
    pub remove: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct KeyframeArgs {
    /// Object id (clip / effect / track / composition).
    pub id: String,
    /// Property path, e.g. "opacity", "transform.scale", "params.amount" (on an effect id) or "effects.fx3.params.amount" (on a clip).
    pub path: String,
    /// Replace all keyframes: [[t, v, ease], ...] or [{t, v, ease}]. Times are clip-local seconds.
    #[serde(default)]
    pub keyframes: Option<Json>,
    /// Add/replace a single keyframe.
    #[serde(default)]
    pub add: Option<SingleKey>,
    /// Set an expression ("" removes it).
    #[serde(default)]
    pub expr: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SingleKey {
    pub t: f64,
    pub v: Json,
    #[serde(default)]
    pub ease: Option<Json>,
}

#[derive(Deserialize, JsonSchema)]
pub struct HistoryArgs {
    /// undo | redo | list
    pub action: String,
    /// How many steps (undo/redo).
    #[serde(default)]
    pub steps: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ImportArgs {
    /// Files, directories or globs (relative to the project folder or absolute).
    pub paths: Vec<String>,
    #[serde(flatten)]
    pub options: ImportOptions,
}

#[derive(Deserialize, JsonSchema)]
pub struct AssetArg {
    pub asset: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct AnalyzeArgs {
    /// Audio (or video with audio) asset id.
    pub asset: String,
    /// Write beats/drops/sections into project timing (default true).
    #[serde(default = "yes")]
    pub apply: bool,
    /// Timeline offset of the music (default: start of the first clip using it).
    #[serde(default)]
    pub offset: Option<f64>,
    /// Known tempo to lock onto.
    #[serde(default)]
    pub bpm_hint: Option<f64>,
    /// Return every beat time (default: first 32 + counts).
    #[serde(default)]
    pub full: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize, JsonSchema)]
pub struct ScenesArgs {
    pub asset: String,
    /// Cut sensitivity 0..1 (default 0.5).
    #[serde(default)]
    pub sensitivity: Option<f64>,
    /// Minimum shot length seconds (default 0.35).
    #[serde(default)]
    pub min_shot: Option<f64>,
    /// Analyze only [start, end] seconds.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
    /// Return a contact sheet image of the shots' representative frames.
    #[serde(default)]
    pub preview: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct MediaPreviewArgs {
    pub asset: String,
    /// Exact source times; otherwise `count` evenly spaced frames.
    #[serde(default)]
    pub times: Option<Vec<f64>>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub end: Option<f64>,
    #[serde(default)]
    pub columns: Option<u32>,
    #[serde(default)]
    pub thumb_width: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct WaveformArgs {
    pub asset: String,
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub end: Option<f64>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TrackAddArgs {
    #[serde(default)]
    pub name: String,
    /// Composition id (default root).
    #[serde(default)]
    pub comp: Option<String>,
    /// Position from the bottom (default: top).
    #[serde(default)]
    pub index: Option<usize>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ClipAddArgs {
    /// One clip (JSON object; see reference "format"). Missing id/start/duration are filled in.
    #[serde(default)]
    pub clip: Option<Json>,
    /// Several clips at once (same track rules).
    #[serde(default)]
    pub clips: Option<Vec<Json>>,
    /// Track id, track index (number), or "new" (default: new track).
    #[serde(default)]
    pub track: Option<Json>,
    /// Composition id (default root).
    #[serde(default)]
    pub comp: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ClipUpdateArgs {
    pub id: String,
    /// Merge patch for the clip (objects merge, null deletes).
    pub patch: Json,
}

#[derive(Deserialize, JsonSchema)]
pub struct ClipOpArgs {
    /// split | move | duplicate | trim | snap
    pub op: String,
    pub id: String,
    /// split: timeline time to cut at.
    #[serde(default)]
    pub time: Option<f64>,
    /// move/duplicate: new start; trim: new start (keeps end) — use end to trim the tail.
    #[serde(default)]
    pub start: Option<f64>,
    /// trim: new end time.
    #[serde(default)]
    pub end: Option<f64>,
    /// move: destination track id.
    #[serde(default)]
    pub track: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RemoveArgs {
    /// Ids of clips/tracks/effects/markers/assets/compositions to delete.
    pub ids: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct EffectAddArgs {
    /// Clip, track or composition id.
    pub target: String,
    /// Effect id (effects_list).
    pub effect: String,
    #[serde(default)]
    pub params: Option<Json>,
    #[serde(default)]
    pub mix: Option<Json>,
    /// Position in the stack (default: last).
    #[serde(default)]
    pub index: Option<usize>,
    /// Only active in this clip-local time range.
    #[serde(default)]
    pub range: Option<[f64; 2]>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListArgs {
    /// filter | transition | generator (effects only).
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    /// Free-text search (id, name, tags, description).
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct InfoArgs {
    pub id: String,
    /// Include the source code (WGSL / Rhai).
    #[serde(default)]
    pub source: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct EffectPreviewArgs {
    /// Effect id.
    pub effect: String,
    /// Param overrides (static values).
    #[serde(default)]
    pub params: Option<serde_json::Map<String, Json>>,
    /// Composition time of the frame to apply it to (default: middle). Empty projects use a test card.
    #[serde(default)]
    pub time: Option<f64>,
    /// Transition progress 0..1 (default 0.5).
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub max_width: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct CreateArgs {
    /// Id to store it under (overrides the header id).
    pub id: String,
    /// Full source with the //! TOML header.
    pub source: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct PresetApplyArgs {
    pub preset: String,
    /// Clip id for clip presets (omit for timeline presets).
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub args: Option<serde_json::Map<String, Json>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ScriptArgs {
    /// Rhai code (see reference "scripting"). The last expression is returned.
    pub code: String,
    #[serde(default)]
    pub args: Option<Json>,
    /// Run without committing changes (returns what would change).
    #[serde(default)]
    pub dry_run: bool,
}

#[derive(Deserialize, JsonSchema)]
pub struct ExprTestArgs {
    pub expr: String,
    /// Clip id providing context (start/duration); default: a 10 s virtual clip at 0.
    #[serde(default)]
    pub clip: Option<String>,
    /// Clip-local times to sample (default 0..duration in 12 steps).
    #[serde(default)]
    pub times: Option<Vec<f64>>,
    /// Value of `value` (keyframed base), default 1.0.
    #[serde(default)]
    pub value: Option<Json>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RenderFrameArgs {
    /// Composition time in seconds.
    pub time: f64,
    #[serde(default)]
    pub comp: Option<String>,
    /// Max width of the returned image (default 1280).
    #[serde(default)]
    pub max_width: Option<u32>,
    /// black | checker | transparent (default black).
    #[serde(default)]
    pub background: Option<String>,
    /// Also save a full-res PNG to this path.
    #[serde(default)]
    pub save: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RenderFramesArgs {
    /// Exact times; otherwise `count` evenly spaced frames over [start, end].
    #[serde(default)]
    pub times: Option<Vec<f64>>,
    #[serde(default)]
    pub count: Option<u32>,
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub end: Option<f64>,
    /// Snap sample times to beats (good for checking hits).
    #[serde(default)]
    pub on_beats: bool,
    #[serde(default)]
    pub comp: Option<String>,
    #[serde(default)]
    pub columns: Option<u32>,
    #[serde(default)]
    pub thumb_width: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ExportArgs {
    #[serde(flatten)]
    pub settings: ExportSettings,
    #[serde(default)]
    pub comp: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct PreviewArgs {
    /// Output path (default: <project>_preview.mp4).
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub start: Option<f64>,
    #[serde(default)]
    pub end: Option<f64>,
    /// Output width (default 640).
    #[serde(default)]
    pub width: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub struct TopicArgs {
    /// workflow | format | expressions | scripting | effects | presets | easings | blend_modes | tips
    #[serde(default)]
    pub topic: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct SchemaArgs {
    /// project | clip | source | transform | effect | transition | mask | text | text_animator | shape | audio | asset | timing | easing | ...
    #[serde(default = "proj")]
    pub r#type: String,
}

fn proj() -> String {
    "project".into()
}

#[derive(Deserialize, JsonSchema)]
pub struct FontArgs {
    #[serde(default)]
    pub query: Option<String>,
}

// ------------------------------------------------------------------------------------------------
// Tools
// ------------------------------------------------------------------------------------------------

#[tool_router]
impl EditsServer {
    pub fn new(engine: Option<Engine>, gpu: edits_render::GpuOptions) -> Self {
        EditsServer { state: Arc::new(Mutex::new(State { engine, gpu })), tool_router: Self::tool_router() }
    }

    async fn run<F>(&self, f: F) -> Result<CallToolResult, McpError>
    where
        F: FnOnce(&mut State) -> Res + Send + 'static,
    {
        let st = self.state.clone();
        match tokio::task::spawn_blocking(move || {
            let mut s = st.lock();
            f(&mut s)
        })
        .await
        {
            Ok(Ok(r)) => Ok(r),
            Ok(Err(e)) => Ok(CallToolResult::error(vec![ContentBlock::text(format!("error: {e:#}"))])),
            Err(e) => Ok(CallToolResult::error(vec![ContentBlock::text(format!("internal error: {e}"))])),
        }
    }

    // ---------------- project ----------------

    #[tool(description = "Open a project file, or create it with create:true (width/height/fps/duration). Returns the project summary.")]
    async fn project_open(&self, Parameters(a): Parameters<OpenArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let path = PathBuf::from(&a.path);
            let mut e = if path.exists() {
                Engine::open(&path)?
            } else if a.create {
                let name = a.name.clone().unwrap_or_else(|| path.file_stem().and_then(|x| x.to_str()).unwrap_or("edit").trim_end_matches(".edits").to_string());
                Engine::create(&path, Project::new(&name, a.width.unwrap_or(1920), a.height.unwrap_or(1080), a.fps.unwrap_or(24.0), a.duration.unwrap_or(30.0)))?
            } else {
                anyhow::bail!("{} does not exist (pass create:true to create it)", path.display());
            };
            e.gpu_options = s.gpu.clone();
            let summ = summary::summarize(&e.project);
            s.engine = Some(e);
            Ok(text(format!("opened {}\n{summ}", a.path)))
        })
        .await
    }

    #[tool(description = "Compact text overview of the whole project: comps, tracks, clips (time ranges, sources, effects, transitions), assets, timing. Cheapest way to see the edit.")]
    async fn project_summary(&self) -> Result<CallToolResult, McpError> {
        self.run(|s| {
            let e = engine(s)?;
            let v = issues_text(e);
            Ok(text(format!("{}\n{v}", summary::summarize(&e.project))))
        })
        .await
    }

    #[tool(description = "Get JSON of the project, of any object by id (clip/track/effect/asset/comp/marker), or of a path inside it.")]
    async fn project_get(&self, Parameters(a): Parameters<GetArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let v = match (&a.id, &a.path) {
                (None, None) => serde_json::to_value(&e.project)?,
                (None, Some(p)) => {
                    let all = serde_json::to_value(&e.project)?;
                    ops::path_get(&all, p).cloned().ok_or_else(|| anyhow::anyhow!("path not found: {p}"))?
                }
                (Some(id), None) => ops::get_object(&e.project, id)?,
                (Some(id), Some(p)) => ops::get_path(&e.project, id, p)?,
            };
            Ok(json_result(&v))
        })
        .await
    }

    #[tool(description = "Patch the project or an object: `merge` (RFC 7396 merge patch, easiest) or `ops` (RFC 6902 JSON Patch). The result is validated; invalid changes are rejected atomically.")]
    async fn project_patch(&self, Parameters(a): Parameters<PatchArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            e.mutate("patch", |p| {
                let mut target = match &a.id {
                    Some(id) => ops::get_object(p, id)?,
                    None => serde_json::to_value(&*p)?,
                };
                if let Some(m) = &a.merge {
                    json_patch::merge(&mut target, m);
                }
                if let Some(o) = &a.ops {
                    let patch: json_patch::Patch = serde_json::from_value(o.clone()).map_err(|e| edits_core::EditError::Invalid(format!("bad JSON Patch: {e}")))?;
                    json_patch::patch(&mut target, &patch).map_err(|e| edits_core::EditError::Invalid(e.to_string()))?;
                }
                match &a.id {
                    Some(id) => ops::set_object(p, id, target),
                    None => {
                        *p = serde_json::from_value(target).map_err(|e| edits_core::EditError::Invalid(format!("invalid project: {e}")))?;
                        Ok(())
                    }
                }
            })?;
            Ok(text(format!("ok\n{}", issues_text(e))))
        })
        .await
    }

    #[tool(description = "Set (or remove=true to reset) one field/property of an object by dot path, e.g. {id:'c3', path:'transform.scale', value:1.2} or {id:'fx2', path:'params.amount', value:{expr:'10*pulse(8.0)'}}.")]
    async fn set(&self, Parameters(a): Parameters<SetArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            if a.remove {
                e.mutate("unset", |p| ops::unset_path(p, &a.id, &a.path))?;
            } else {
                e.mutate("set", |p| ops::set_path(p, &a.id, &a.path, a.value.clone()))?;
            }
            let v = ops::get_path(&e.project, &a.id, &a.path).unwrap_or(Json::Null);
            Ok(text(format!("{}.{} = {}", a.id, a.path, serde_json::to_string(&v)?)))
        })
        .await
    }

    #[tool(description = "Animate a property: replace keyframes, add one keyframe, and/or set an expression. Times are clip-local seconds; ease on a key shapes the segment to the next key.")]
    async fn keyframes(&self, Parameters(a): Parameters<KeyframeArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            e.mutate("keyframes", |p| {
                if let Some(k) = &a.keyframes {
                    ops::set_keyframes(p, &a.id, &a.path, k.clone())?;
                }
                if let Some(k) = &a.add {
                    let v: edits_core::Value = serde_json::from_value(k.v.clone())?;
                    let ease: edits_core::Easing = match &k.ease {
                        Some(e) => serde_json::from_value(e.clone())?,
                        None => edits_core::Easing::Linear,
                    };
                    ops::add_keyframe(p, &a.id, &a.path, k.t, v, ease)?;
                }
                if let Some(x) = &a.expr {
                    ops::set_expression(p, &a.id, &a.path, if x.is_empty() { None } else { Some(x.clone()) })?;
                }
                Ok(())
            })?;
            let v = ops::get_path(&e.project, &a.id, &a.path).unwrap_or(Json::Null);
            let mut msg = format!("{}.{} = {}", a.id, a.path, serde_json::to_string(&v)?);
            if let Some(x) = a.expr.as_ref().filter(|x| !x.is_empty()) {
                if let Err(err) = e.expr.compile(x) {
                    msg.push_str(&format!("\nwarning: expression does not compile: {err}"));
                }
            }
            Ok(text(msg))
        })
        .await
    }

    #[tool(description = "Undo / redo changes, or list the history (action: undo | redo | list).")]
    async fn history(&self, Parameters(a): Parameters<HistoryArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let n = a.steps.unwrap_or(1).max(1);
            match a.action.as_str() {
                "undo" => {
                    let mut done = vec![];
                    for _ in 0..n {
                        match e.undo()? {
                            Some(l) => done.push(l),
                            None => break,
                        }
                    }
                    Ok(text(format!("undone: {done:?}")))
                }
                "redo" => {
                    let mut done = vec![];
                    for _ in 0..n {
                        match e.redo()? {
                            Some(l) => done.push(l),
                            None => break,
                        }
                    }
                    Ok(text(format!("redone: {done:?}")))
                }
                _ => {
                    let (u, r) = e.history.labels();
                    Ok(json_result(&json!({"undo": u, "redo": r, "revision": e.history.revision})))
                }
            }
        })
        .await
    }

    // ---------------- media ----------------

    #[tool(description = "Import media: files, folders (recursive optional) or globs. Kinds are auto-detected: video, audio, image, animated_image (gif/webp/apng), svg, image_sequence (pattern with %04d or a folder with kind), font, lut (.cube), subtitles (.lrc/.srt/.vtt/.ass), data. Ids come from file names.")]
    async fn media_import(&self, Parameters(a): Parameters<ImportArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let list = e.import(&a.paths, &a.options)?;
            let rows: Vec<Json> = list
                .iter()
                .map(|x| {
                    let i = x.info.clone().unwrap_or_default();
                    json!({"id": x.id, "kind": x.kind, "path": x.path, "size": if i.width > 0 { format!("{}x{}", i.width, i.height) } else { String::new() },
                           "duration": i.duration, "fps": i.fps, "audio": i.has_audio, "fonts": i.families})
                })
                .collect();
            Ok(json_result(&rows))
        })
        .await
    }

    #[tool(description = "Asset details (metadata, path, tags).")]
    async fn media_info(&self, Parameters(a): Parameters<AssetArg>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            Ok(json_result(&ops::get_object(&e.project, &a.asset)?))
        })
        .await
    }

    #[tool(description = "Analyze music: tempo, beats, downbeats, drops, sections, accents and loudness envelopes (cached). apply=true (default) writes them to project timing so beat-synced presets and expressions (pulse(), bass()...) work.")]
    async fn analyze_audio(&self, Parameters(a): Parameters<AnalyzeArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let opts = AnalysisOptions { bpm_hint: a.bpm_hint, ..Default::default() };
            let an = e.analyze_audio(&a.asset, &opts, a.apply, a.offset)?;
            let beats: Vec<f64> = if a.full { an.beats.clone() } else { an.beats.iter().take(32).copied().collect() };
            Ok(json_result(&json!({
                "bpm": an.bpm, "tempo_confidence": (an.tempo_confidence * 100.0).round() / 100.0, "duration": an.duration,
                "beat_count": an.beats.len(), "beats": beats, "downbeats": if a.full { an.downbeats.clone() } else { an.downbeats.iter().take(16).copied().collect() },
                "drops": an.drops, "sections": an.sections, "accents_count": an.accents.len(),
                "applied_to_timing": a.apply, "offset": e.project.timing.offset,
            })))
        })
        .await
    }

    #[tool(description = "Detect shots/scene cuts in a video asset with per-shot motion, brightness, saturation and average color (cached). preview:true also returns a contact sheet of the shots.")]
    async fn detect_scenes(&self, Parameters(a): Parameters<ScenesArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let mut o = SceneOptions::default();
            if let Some(x) = a.sensitivity {
                o.sensitivity = x;
            }
            if let Some(x) = a.min_shot {
                o.min_shot = x;
            }
            o.range = a.range;
            let shots = e.detect_scenes(&a.asset, &o)?;
            let rows: Vec<String> = shots
                .iter()
                .map(|s| format!("#{} {:.2}-{:.2} ({:.2}s) motion={:.2} bright={:.2} sat={:.2} {}", s.index, s.start, s.end, s.end - s.start, s.motion, s.brightness, s.saturation, s.color))
                .collect();
            let mut out = vec![ContentBlock::text(format!("{} shots\n{}", shots.len(), rows.join("\n")))];
            if a.preview && !shots.is_empty() {
                let times: Vec<f64> = shots.iter().take(48).map(|s| s.thumb).collect();
                let sheet = e.asset_sheet(&a.asset, &times, 6, 200)?;
                out.push(image_block(&sheet, 1400, true)?);
            }
            Ok(CallToolResult::success(out))
        })
        .await
    }

    #[tool(description = "See an asset: contact sheet of a video/image/gif at given times (or evenly spaced), or a waveform with beats for audio.")]
    async fn media_preview(&self, Parameters(a): Parameters<MediaPreviewArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let asset = e.project.assets.get(&a.asset).cloned().ok_or_else(|| anyhow::anyhow!("asset '{}' not found", a.asset))?;
            if asset.kind == edits_core::AssetKind::Audio {
                let wf = e.waveform_image(&a.asset, a.start.unwrap_or(0.0), a.end, 1200, 240)?;
                return Ok(CallToolResult::success(vec![image_block(&wf, 1200, false)?]));
            }
            let dur = asset.info.as_ref().and_then(|i| i.duration).unwrap_or(0.0);
            let times = a.times.clone().unwrap_or_else(|| {
                let n = a.count.unwrap_or(12).clamp(1, 64);
                let (t0, t1) = (a.start.unwrap_or(0.0), a.end.unwrap_or(dur).max(a.start.unwrap_or(0.0)));
                (0..n).map(|i| t0 + (t1 - t0) * (i as f64 + 0.5) / n as f64).collect()
            });
            let sheet = e.asset_sheet(&a.asset, &times, a.columns.unwrap_or(4), a.thumb_width.unwrap_or(320))?;
            Ok(CallToolResult::success(vec![ContentBlock::text(format!("times: {times:?}")), image_block(&sheet, 1600, true)?]))
        })
        .await
    }

    #[tool(description = "Waveform image of an audio asset with beat grid (gray), downbeats (white), drops (red) and section bands.")]
    async fn waveform(&self, Parameters(a): Parameters<WaveformArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let wf = e.waveform_image(&a.asset, a.start.unwrap_or(0.0), a.end, a.width.unwrap_or(1200), a.height.unwrap_or(240))?;
            Ok(CallToolResult::success(vec![image_block(&wf, 2000, false)?]))
        })
        .await
    }

    #[tool(description = "List available font families (system + imported), optionally filtered.")]
    async fn fonts_list(&self, Parameters(a): Parameters<FontArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let q = a.query.map(|q| q.to_lowercase());
            let fams: Vec<String> = e.text.families().into_iter().filter(|f| q.as_ref().is_none_or(|q| f.to_lowercase().contains(q))).collect();
            Ok(text(format!("{} families:\n{}", fams.len(), fams.join(", "))))
        })
        .await
    }

    // ---------------- timeline ----------------

    #[tool(description = "Add a track to a composition (bottom→top order; default on top).")]
    async fn track_add(&self, Parameters(a): Parameters<TrackAddArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let id = e.mutate("add track", |p| ops::add_track(p, a.comp.as_deref(), &a.name, a.index))?;
            Ok(text(format!("track {id}")))
        })
        .await
    }

    #[tool(description = "Add clip(s). Pass `clip` or `clips` (JSON per reference 'format'). track: id, index, or 'new' (default). Missing start = end of the track's last clip; missing duration = media length / speed (or 3 s).")]
    async fn clip_add(&self, Parameters(a): Parameters<ClipAddArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let mut list = a.clips.clone().unwrap_or_default();
            if let Some(c) = &a.clip {
                list.insert(0, c.clone());
            }
            if list.is_empty() {
                anyhow::bail!("pass `clip` or `clips`");
            }
            let ids = e.mutate("add clips", |p| {
                let mut target = match &a.track {
                    None => ops::TrackTarget::New,
                    Some(Json::String(t)) if t == "new" || t.is_empty() => ops::TrackTarget::New,
                    Some(Json::String(t)) => ops::TrackTarget::Id(t.clone()),
                    Some(Json::Number(n)) => ops::TrackTarget::Index(n.as_u64().unwrap_or(0) as usize),
                    Some(other) => return Err(edits_core::EditError::Invalid(format!("bad track {other}"))),
                };
                let mut ids = vec![];
                for c in &list {
                    let id = ops::add_clip(p, a.comp.as_deref(), target.clone(), c.clone())?;
                    // batch into the same new track
                    if matches!(target, ops::TrackTarget::New) {
                        if let Some(loc) = p.find_clip(&id) {
                            target = ops::TrackTarget::Id(p.compositions[&loc.comp].tracks[loc.track].id.clone());
                        }
                    }
                    ids.push(id);
                }
                Ok(ids)
            })?;
            let lines: Vec<String> = ids
                .iter()
                .filter_map(|id| e.project.clip(id).map(|c| format!("{} [{:.3}-{:.3}] {}", c.id, c.start, c.end(), c.source.kind_name())))
                .collect();
            Ok(text(lines.join("\n")))
        })
        .await
    }

    #[tool(description = "Update a clip with a merge patch, e.g. {id:'c4', patch:{opacity:0.8, blend_mode:'screen', transform:{rotation:10}}}.")]
    async fn clip_update(&self, Parameters(a): Parameters<ClipUpdateArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            e.mutate("update clip", |p| ops::merge_object(p, &a.id, &a.patch))?;
            Ok(text(format!("updated {}", a.id)))
        })
        .await
    }

    #[tool(description = "Clip operations: split (at time), move (start, track), duplicate (start), trim (start and/or end, adjusting the in-point), snap (start to the nearest beat).")]
    async fn clip_op(&self, Parameters(a): Parameters<ClipOpArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let msg = e.mutate(&format!("clip {}", a.op), |p| -> edits_core::Result<String> {
                match a.op.as_str() {
                    "split" => Ok(format!("new clip {}", ops::split_clip(p, &a.id, a.time.ok_or_else(|| edits_core::EditError::Invalid("split needs time".into()))?)?)),
                    "move" => {
                        ops::move_clip(p, &a.id, a.start, a.track.as_deref())?;
                        Ok("moved".into())
                    }
                    "duplicate" => Ok(format!("new clip {}", ops::duplicate_clip(p, &a.id, a.start)?)),
                    "trim" => {
                        let c = p.clip_mut(&a.id).ok_or_else(|| edits_core::EditError::NotFound(a.id.clone()))?;
                        let end = a.end.unwrap_or(c.end());
                        if let Some(st) = a.start {
                            let delta = st - c.start;
                            let consumed = c.integrate_speed(delta.max(0.0));
                            c.source_in += if delta >= 0.0 { consumed } else { delta };
                            c.source_in = c.source_in.max(0.0);
                            c.start = st;
                        }
                        c.duration = (end - c.start).max(0.01);
                        Ok(format!("trimmed to [{:.3}-{:.3}]", c.start, c.end()))
                    }
                    "snap" => {
                        let st = p.clip(&a.id).map(|c| c.start).ok_or_else(|| edits_core::EditError::NotFound(a.id.clone()))?;
                        let snapped = ops::snap_to_beat(p, st, 1.0);
                        ops::move_clip(p, &a.id, Some(snapped), None)?;
                        Ok(format!("start {st:.3} → {snapped:.3}"))
                    }
                    other => Err(edits_core::EditError::Invalid(format!("unknown op '{other}'"))),
                }
            })?;
            Ok(text(msg))
        })
        .await
    }

    #[tool(description = "Delete objects by id (clips, tracks, effects, markers, assets, compositions).")]
    async fn remove(&self, Parameters(a): Parameters<RemoveArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let kinds = e.mutate("remove", |p| a.ids.iter().map(|id| ops::remove(p, id).map(|k| format!("{k} {id}"))).collect::<edits_core::Result<Vec<_>>>())?;
            Ok(text(format!("removed: {}", kinds.join(", "))))
        })
        .await
    }

    #[tool(description = "Add an effect to a clip, track or composition. params values can be numbers, colors, arrays, keyframes or {expr}. See effect_info for params.")]
    async fn effect_add(&self, Parameters(a): Parameters<EffectAddArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let lib = e.library();
            let def = lib.effect(&a.effect).ok_or_else(|| anyhow::anyhow!("unknown effect '{}' — try effects_list {{query}}", a.effect))?;
            if def.kind != EffectKind::Filter {
                anyhow::bail!("'{}' is a {}: use it as {}", def.id, def.kind.as_str(), if def.kind == EffectKind::Transition { "a clip's transition_in" } else { "a generator clip source" });
            }
            let mut fx = EffectInstance::new("", &a.effect);
            if let Some(p) = &a.params {
                fx.params = serde_json::from_value(p.clone())?;
                for k in fx.params.keys() {
                    if def.param(k).is_none() {
                        anyhow::bail!("effect '{}' has no param '{k}'. Params: {}", def.id, def.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "));
                    }
                }
            }
            if let Some(m) = &a.mix {
                fx.mix = serde_json::from_value(m.clone())?;
            }
            fx.range = a.range;
            let id = e.mutate("add effect", |p| ops::add_effect(p, &a.target, fx, a.index))?;
            Ok(text(format!("effect {id} ({}) on {}", a.effect, a.target)))
        })
        .await
    }

    // ---------------- library ----------------

    #[tool(description = "List effects (filters, transitions, generators) with one-line descriptions. Filter by kind/category/query. 169+ built in.")]
    async fn effects_list(&self, Parameters(a): Parameters<ListArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let lib = match s.engine.as_mut() {
                Some(e) => e.library(),
                None => Arc::new(edits_fx::Library::builtin().clone()),
            };
            let kind = match a.kind.as_deref() {
                Some("filter") => Some(EffectKind::Filter),
                Some("transition") => Some(EffectKind::Transition),
                Some("generator") => Some(EffectKind::Generator),
                _ => None,
            };
            let list = lib.search_effects(kind, a.category.as_deref(), a.query.as_deref());
            let mut out = String::new();
            let mut last = String::new();
            for e in &list {
                let key = format!("{} / {}", e.kind.as_str(), e.category);
                if key != last {
                    out.push_str(&format!("\n## {key}\n"));
                    last = key;
                }
                out.push_str(&format!("- {}: {} [{}]\n", e.id, e.description, e.params.join(", ")));
            }
            Ok(text(format!("{} effects{}", list.len(), out)))
        })
        .await
    }

    #[tool(description = "Effect details: kind, description, params (type, default, range, options), passes; source:true includes the WGSL.")]
    async fn effect_info(&self, Parameters(a): Parameters<InfoArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let lib = match s.engine.as_mut() {
                Some(e) => e.library(),
                None => Arc::new(edits_fx::Library::builtin().clone()),
            };
            let d = lib.effect(&a.id).ok_or_else(|| anyhow::anyhow!("unknown effect '{}'", a.id))?;
            let mut v = serde_json::to_value(&**d)?;
            if a.source {
                v["source"] = Json::String(d.source.clone());
            }
            Ok(json_result(&v))
        })
        .await
    }

    #[tool(description = "See what an effect does: applies it (with optional params) to a frame of your edit — or a test card — and returns the image. Works for filters, transitions (at progress) and generators.")]
    async fn effect_preview(&self, Parameters(a): Parameters<EffectPreviewArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = match s.engine.as_mut() {
                Some(e) => e,
                None => {
                    s.engine = Some(Engine::new(Project::new("preview", 960, 540, 24.0, 4.0), None));
                    s.engine.as_mut().unwrap()
                }
            };
            let f = e.preview_effect(&a.effect, &a.params.clone().unwrap_or_default(), a.time, a.progress.unwrap_or(0.5))?;
            Ok(CallToolResult::success(vec![ContentBlock::text(format!("{} preview", a.effect)), image_block(&f, a.max_width.unwrap_or(960), true)?]))
        })
        .await
    }

    #[tool(description = "Create or replace a custom WGSL effect stored in the project (same format as built-ins; see reference 'effects'). Validated with naga and compiled on the GPU; errors cite your line numbers.")]
    async fn effect_create(&self, Parameters(a): Parameters<CreateArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let mut def = EffectDef::parse(&a.source, &a.id, false)?;
            def.id = a.id.clone();
            def.validate()?;
            if let Ok(r) = e.renderer() {
                r.prepare_effect(&def)?;
            }
            e.mutate("create effect", |p| {
                p.custom_effects.insert(a.id.clone(), a.source.clone());
                Ok(())
            })?;
            Ok(text(format!("effect '{}' ({}) ready with params: {}", a.id, def.kind.as_str(), def.params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))))
        })
        .await
    }

    #[tool(description = "List presets (Rhai recipes: impacts, velocity, entrances, looks, text animations, audio-reactive, timeline builders like auto_amv / beat_cut_montage / lyrics_from_subtitles).")]
    async fn presets_list(&self, Parameters(a): Parameters<ListArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let lib = match s.engine.as_mut() {
                Some(e) => e.library(),
                None => Arc::new(edits_fx::Library::builtin().clone()),
            };
            let list = lib.search_presets(a.category.as_deref(), a.query.as_deref());
            let mut out = String::new();
            let mut last = String::new();
            for p in &list {
                if p.category != last {
                    out.push_str(&format!("\n## {}\n", p.category));
                    last = p.category.clone();
                }
                out.push_str(&format!("- {} ({}): {} [{}]\n", p.id, if p.scope == edits_fx::PresetScope::Clip { "clip" } else { "timeline" }, p.description, p.params.join(", ")));
            }
            Ok(text(format!("{} presets{}", list.len(), out)))
        })
        .await
    }

    #[tool(description = "Preset details: scope, params with defaults; source:true includes the Rhai code (copy & modify it with preset_create).")]
    async fn preset_info(&self, Parameters(a): Parameters<InfoArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let lib = match s.engine.as_mut() {
                Some(e) => e.library(),
                None => Arc::new(edits_fx::Library::builtin().clone()),
            };
            let d = lib.preset(&a.id).ok_or_else(|| anyhow::anyhow!("unknown preset '{}'", a.id))?;
            let mut v = serde_json::to_value(&**d)?;
            if a.source {
                v["source"] = Json::String(d.source.clone());
            }
            Ok(json_result(&v))
        })
        .await
    }

    #[tool(description = "Apply a preset. Clip presets need target (clip id); time params like `at` are clip-local seconds. Timeline presets build or modify the whole edit.")]
    async fn preset_apply(&self, Parameters(a): Parameters<PresetApplyArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let args = a.args.clone().unwrap_or_default();
            let rep = e.apply_preset(&a.preset, a.target.as_deref(), &args, None)?;
            let mut msg = format!("preset {} applied (changed: {})", a.preset, rep.changed);
            if !rep.logs.is_empty() {
                msg.push_str(&format!("\nlog: {}", rep.logs.join(" | ")));
            }
            if !rep.result.is_null() {
                msg.push_str(&format!("\nresult: {}", rep.result));
            }
            Ok(text(msg))
        })
        .await
    }

    #[tool(description = "Create or replace a custom preset (Rhai with //! TOML header; see reference 'scripting'). It becomes available to preset_apply and to other presets.")]
    async fn preset_create(&self, Parameters(a): Parameters<CreateArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let def = PresetDef::parse(&a.source, &a.id, false)?;
            edits_engine::script::check_script(&a.source).map_err(|err| anyhow::anyhow!("script does not compile: {err}"))?;
            e.mutate("create preset", |p| {
                p.custom_presets.insert(a.id.clone(), a.source.clone());
                Ok(())
            })?;
            Ok(text(format!("preset '{}' ({:?}) ready", a.id, def.scope)))
        })
        .await
    }

    // ---------------- code ----------------

    #[tool(description = "Run Rhai code against the project (one undoable step). Build timelines procedurally: loop over beats(), scenes(asset), subtitles(asset); call project.add_clip/add_effect/apply_preset... dry_run previews without committing.")]
    async fn script_run(&self, Parameters(a): Parameters<ScriptArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let before = summary::summarize(&e.project);
            let (rep, preview) = e.run_script(&a.code, a.args.clone().unwrap_or(Json::Null), a.dry_run)?;
            let after = match &preview {
                Some(p) => summary::summarize(p),
                None => summary::summarize(&e.project),
            };
            let mut msg = format!("{} (changed: {})", if a.dry_run { "dry run" } else { "applied" }, rep.changed);
            if !rep.result.is_null() {
                msg.push_str(&format!("\nresult: {}", rep.result));
            }
            if !rep.logs.is_empty() {
                msg.push_str(&format!("\nlog:\n{}", rep.logs.join("\n")));
            }
            if rep.changed && before != after {
                msg.push_str(&format!("\n--- project after ---\n{after}"));
            }
            Ok(text(msg))
        })
        .await
    }

    #[tool(description = "Evaluate an expression at several clip-local times (debug beat/audio-reactive expressions before attaching them).")]
    async fn expression_test(&self, Parameters(a): Parameters<ExprTestArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let timing = e.timing_data();
            let comp = e.project.root_comp().cloned().ok_or_else(|| anyhow::anyhow!("no root comp"))?;
            let (start, dur) = match &a.clip {
                Some(id) => e.project.clip(id).map(|c| (c.start, c.duration)).ok_or_else(|| anyhow::anyhow!("clip not found"))?,
                None => (0.0, 10.0),
            };
            let base: edits_core::Value = match &a.value {
                Some(v) => serde_json::from_value(v.clone())?,
                None => edits_core::Value::Num(1.0),
            };
            let times = a.times.clone().unwrap_or_else(|| (0..12).map(|i| dur * i as f64 / 11.0).collect());
            let mut rows = vec![];
            for t in times {
                let ctx = edits_engine::expr::ExprCtx {
                    comp_time: start + t,
                    root_time: start + t,
                    clip_start: start,
                    duration: dur,
                    fps: comp.fps,
                    width: comp.width as f64,
                    height: comp.height as f64,
                    seed: 1,
                    timing: timing.clone(),
                    variables: Arc::new(e.project.variables.clone()),
                };
                match e.expr.eval(&a.expr, &ctx, t, &base) {
                    Ok(v) => rows.push(format!("t={t:.3} → {}", serde_json::to_string(&v)?)),
                    Err(err) => {
                        rows.push(format!("error: {err}"));
                        break;
                    }
                }
            }
            Ok(text(rows.join("\n")))
        })
        .await
    }

    // ---------------- rendering ----------------

    #[tool(description = "Render one frame of the edit and SEE it (returns an image). Also returns render warnings (missing assets, unknown effects, expression errors).")]
    async fn render_frame(&self, Parameters(a): Parameters<RenderFrameArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let mode = match a.background.as_deref() {
                Some("checker") => OutputMode::Checker,
                Some("transparent") => OutputMode::Straight,
                _ => OutputMode::Over([0.0, 0.0, 0.0, 1.0]),
            };
            let r = e.render_frame(a.comp.as_deref(), a.time, mode)?;
            if let Some(p) = &a.save {
                std::fs::write(p, r.frame.to_png()?)?;
            }
            let mut info = format!("t={:.3}s rendered in {:.0} ms ({}x{})", a.time, r.ms, r.frame.width, r.frame.height);
            if !r.warnings.is_empty() {
                info.push_str(&format!("\nwarnings:\n- {}", r.warnings.join("\n- ")));
            }
            Ok(CallToolResult::success(vec![ContentBlock::text(info), image_block(&r.frame, a.max_width.unwrap_or(1280), a.background.as_deref() != Some("transparent"))?]))
        })
        .await
    }

    #[tool(description = "Render many frames into one labeled contact sheet image — the fastest way to review an edit (count evenly spaced, explicit times, or on_beats).")]
    async fn render_frames(&self, Parameters(a): Parameters<RenderFramesArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let comp = e.project.compositions.get(a.comp.as_deref().unwrap_or(&e.project.root)).cloned().ok_or_else(|| anyhow::anyhow!("comp not found"))?;
            let (t0, t1) = (a.start.unwrap_or(0.0), a.end.unwrap_or(comp.duration));
            let mut times = a.times.clone().unwrap_or_else(|| {
                let n = a.count.unwrap_or(12).clamp(1, 48);
                (0..n).map(|i| t0 + (t1 - t0) * (i as f64 + 0.5) / n as f64).collect()
            });
            if a.on_beats {
                let td = e.timing_data();
                let n = a.count.unwrap_or(12).clamp(1, 48) as usize;
                let beats: Vec<f64> = td.beats.iter().copied().filter(|b| *b >= t0 && *b < t1).collect();
                if !beats.is_empty() {
                    let step = (beats.len() as f64 / n as f64).max(1.0);
                    times = (0..n).map(|i| beats[((i as f64 * step) as usize).min(beats.len() - 1)] + 0.5 / comp.fps).collect();
                    times.dedup();
                }
            }
            let (sheet, warnings) = e.contact_sheet(a.comp.as_deref(), &times, a.columns.unwrap_or(4), a.thumb_width.unwrap_or(400))?;
            let mut info = format!("{} frames: {:?}", times.len(), times.iter().map(|t| (t * 100.0).round() / 100.0).collect::<Vec<_>>());
            if !warnings.is_empty() {
                info.push_str(&format!("\nwarnings:\n- {}", warnings.join("\n- ")));
            }
            Ok(CallToolResult::success(vec![ContentBlock::text(info), image_block(&sheet, 1800, true)?]))
        })
        .await
    }

    #[tool(description = "Render a quick low-res MP4 preview (with audio) for humans to watch. Returns the file path.")]
    async fn render_preview(&self, Parameters(a): Parameters<PreviewArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let out = a.output.clone().unwrap_or_else(|| {
                e.path.as_ref().map(|p| p.with_extension("").with_extension("preview.mp4").display().to_string()).unwrap_or_else(|| "preview.mp4".into())
            });
            let mut st = ExportSettings::new(out);
            st.quality = 45.0;
            st.width = Some(a.width.unwrap_or(640));
            if a.start.is_some() || a.end.is_some() {
                st.range = Some([a.start.unwrap_or(0.0), a.end.unwrap_or(f64::MAX)]);
            }
            let rep = e.export(None, &st, None, |_| {})?;
            Ok(json_result(&rep))
        })
        .await
    }

    #[tool(description = "Export the final video. Codec from extension or `codec` (h264/h265/av1 with NVENC/AMF/QSV auto, prores, prores4444 alpha, vp9 alpha webm, gif, webp, apng, png_sequence) or audio-only (.mp3/.wav/.flac). quality 0-100, optional width/height/range.")]
    async fn export(&self, Parameters(a): Parameters<ExportArgs>) -> Result<CallToolResult, McpError> {
        self.run(move |s| {
            let e = engine(s)?;
            let errors: Vec<String> = e.validate().into_iter().filter(|i| i.severity == Severity::Error).map(|i| i.message).collect();
            if !errors.is_empty() {
                anyhow::bail!("fix validation errors first: {}", errors.join("; "));
            }
            let rep = e.export(a.comp.as_deref(), &a.settings, None, |_| {})?;
            Ok(json_result(&rep))
        })
        .await
    }

    #[tool(description = "Validate the project: structure, missing files, unknown effects/params, broken expressions, custom shader/preset errors.")]
    async fn validate(&self) -> Result<CallToolResult, McpError> {
        self.run(|s| {
            let e = engine(s)?;
            Ok(text(issues_text(e)))
        })
        .await
    }

    #[tool(description = "Engine status: project, revision, GPU, FFmpeg, library sizes.")]
    async fn status(&self) -> Result<CallToolResult, McpError> {
        self.run(|s| {
            let mut v = json!({"project_open": s.engine.is_some()});
            if let Some(e) = s.engine.as_mut() {
                v = e.status();
                v["gpu"] = json!(e.gpu_info().unwrap_or_else(|err| format!("unavailable: {err}")));
            }
            Ok(json_result(&v))
        })
        .await
    }

    #[tool(description = "Documentation for agents. Topics: workflow, format, expressions, scripting, effects (WGSL authoring), presets, easings, blend_modes, tips.")]
    async fn reference(&self, Parameters(a): Parameters<TopicArgs>) -> Result<CallToolResult, McpError> {
        Ok(match reference::topic(&a.topic) {
            Some(t) => text(t),
            None => text(format!("unknown topic; available: {}", reference::TOPICS.join(", "))),
        })
    }

    #[tool(description = "JSON schema of the project file or a sub-type (clip, source, transform, effect, transition, mask, text, text_animator, shape, audio, audio_effect, asset, timing, easing, blend_mode, echo, marker, property).")]
    async fn schema(&self, Parameters(a): Parameters<SchemaArgs>) -> Result<CallToolResult, McpError> {
        Ok(match edits_core::type_schema(&a.r#type) {
            Some(s) => text(serde_json::to_string(&s).unwrap_or_default()),
            None => text(format!("unknown type; available: {}", edits_core::SCHEMA_TYPES.join(", "))),
        })
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for EditsServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("edits-editor", env!("CARGO_PKG_VERSION")).with_title("EditsEditor"))
            .with_instructions(reference::INSTRUCTIONS)
    }
}
