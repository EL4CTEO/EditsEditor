//! The editing session: owns the project, history, media, GPU renderer, analysis caches.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use edits_audio::{AnalysisOptions, AudioAnalysis};
use edits_core::{Asset, AssetKind, EditError, Project, history::History, validate::Issue};
use edits_fx::Library;
use edits_media::{
    Ffmpeg, Frame,
    scenes::{SceneOptions, Shot},
};
use edits_render::{GpuContext, GpuOptions, OutputMode, Renderer, TextRenderer};
use serde::Serialize;
use serde_json::{Value as Json, json};

use crate::{
    EngineError, Result,
    eval::EvalCtx,
    expr::{ExprEngine, TimingData},
    media_pool::{MediaPool, key_of},
    script::{self, ScriptServices},
};

pub struct Engine {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub history: History,
    library: Arc<Library>,
    library_key: u64,
    pub media: MediaPool,
    pub(crate) renderer: Option<Renderer>,
    pub gpu_options: GpuOptions,
    pub text: Arc<TextRenderer>,
    pub expr: Arc<ExprEngine>,
    analyses: HashMap<String, Arc<AudioAnalysis>>,
    scenes: HashMap<String, Arc<Vec<Shot>>>,
    loaded_fonts: HashSet<PathBuf>,
    /// Save the project file after every change.
    pub autosave: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct RenderedFrame {
    #[serde(skip)]
    pub frame: Frame,
    pub time: f64,
    pub ms: f64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, serde::Deserialize, schemars::JsonSchema)]
pub struct ImportOptions {
    /// Force a kind instead of auto-detecting.
    #[serde(default)]
    pub kind: Option<AssetKind>,
    /// Recurse into directories.
    #[serde(default)]
    pub recursive: bool,
    /// Explicit id (single file imports only).
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Frame rate for image sequences.
    #[serde(default)]
    pub sequence_fps: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ImportedAsset {
    pub id: String,
    pub kind: AssetKind,
    pub path: String,
    pub info: Option<edits_core::MediaInfo>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScriptReport {
    pub logs: Vec<String>,
    pub result: Json,
    pub changed: bool,
}

pub(crate) struct EngineServices {
    ff: Option<Ffmpeg>,
    base_dir: PathBuf,
    cache_dir: PathBuf,
}

impl ScriptServices for EngineServices {
    fn scenes(&self, project: &Project, asset: &str) -> std::result::Result<Json, String> {
        let a = project.assets.get(asset).ok_or_else(|| format!("asset '{asset}' not found"))?;
        let shots = detect_scenes_cached(self.ff.as_ref(), &self.base_dir, &self.cache_dir, a, &SceneOptions::default())
            .map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(&*shots).unwrap_or_default())
    }
    fn subtitles(&self, project: &Project, asset: &str) -> std::result::Result<Json, String> {
        let a = project.assets.get(asset).ok_or_else(|| format!("asset '{asset}' not found"))?;
        let p = resolve(&self.base_dir, &a.path);
        let text = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
        let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();
        Ok(serde_json::to_value(edits_media::subtitles::parse(&text, &ext)).unwrap_or_default())
    }
}

fn resolve(base: &Path, p: &str) -> PathBuf {
    let pp = Path::new(p);
    if pp.is_absolute() { pp.to_path_buf() } else { base.join(pp) }
}

fn file_key(path: &Path, extra: &str) -> String {
    let meta = std::fs::metadata(path).ok();
    let mtime = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size = meta.map(|m| m.len()).unwrap_or(0);
    blake3::hash(format!("{}|{size}|{mtime}|{extra}", path.display()).as_bytes()).to_hex()[..20].to_string()
}

fn detect_scenes_cached(ff: Option<&Ffmpeg>, base: &Path, cache: &Path, a: &Asset, opts: &SceneOptions) -> Result<Arc<Vec<Shot>>> {
    let p = resolve(base, &a.path);
    let key = file_key(&p, &serde_json::to_string(opts).unwrap_or_default());
    let cfile = cache.join(format!("scenes-{key}.json"));
    if let Ok(s) = std::fs::read_to_string(&cfile)
        && let Ok(v) = serde_json::from_str::<Vec<Shot>>(&s)
    {
        return Ok(Arc::new(v));
    }
    if a.kind != AssetKind::Video {
        return Err(EngineError::Invalid(format!("scene detection needs a video asset ({:?} given)", a.kind)));
    }
    let ff = ff.ok_or(EngineError::Media(edits_media::MediaError::FfmpegMissing))?;
    let fps = a.info.as_ref().and_then(|i| i.fps).unwrap_or(24.0);
    let shots = edits_media::scenes::detect_scenes(ff, &p, fps, opts)?;
    let _ = std::fs::create_dir_all(cache);
    let _ = std::fs::write(&cfile, serde_json::to_string(&shots).unwrap_or_default());
    Ok(Arc::new(shots))
}

fn sanitize_id(stem: &str) -> String {
    let mut s: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|x| !x.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if s.is_empty() {
        s = "asset".into();
    }
    if s.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        s = format!("a_{s}");
    }
    s.truncate(40);
    s
}

impl Engine {
    pub fn new(project: Project, path: Option<PathBuf>) -> Engine {
        let base =
            path.as_ref().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let ff = Ffmpeg::locate().ok();
        if ff.is_none() {
            tracing::warn!("FFmpeg not found: video/audio import and export are unavailable");
        }
        let mut e = Engine {
            project,
            path,
            history: History::default(),
            library: Arc::new(Library::builtin().clone()),
            library_key: 0,
            media: MediaPool::new(ff, base),
            renderer: None,
            gpu_options: GpuOptions::default(),
            text: Arc::new(TextRenderer::new()),
            expr: Arc::new(ExprEngine::new()),
            analyses: HashMap::new(),
            scenes: HashMap::new(),
            loaded_fonts: HashSet::new(),
            autosave: true,
        };
        e.refresh_library();
        e
    }

    pub fn open(path: &Path) -> Result<Engine> {
        let text = std::fs::read_to_string(path).map_err(|e| EngineError::Invalid(format!("cannot read {}: {e}", path.display())))?;
        let project: Project =
            serde_json::from_str(&text).map_err(|e| EngineError::Invalid(format!("invalid project file {}: {e}", path.display())))?;
        let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        Ok(Engine::new(project, Some(abs)))
    }

    /// Create a new project file (does not overwrite an existing one).
    pub fn create(path: &Path, project: Project) -> Result<Engine> {
        if path.exists() {
            return Err(EngineError::Invalid(format!("{} already exists (open it instead)", path.display())));
        }
        if let Some(d) = path.parent()
            && !d.as_os_str().is_empty()
        {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(&project)?)?;
        let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        Ok(Engine::new(project, Some(abs)))
    }

    pub fn base_dir(&self) -> PathBuf {
        self.media.base_dir.clone()
    }

    pub fn cache_dir(&self) -> PathBuf {
        match &self.path {
            Some(p) => p.parent().unwrap_or(Path::new(".")).join(".edits-cache"),
            None => dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("edits-editor"),
        }
    }

    pub fn save(&mut self) -> Result<()> {
        if let Some(p) = &self.path {
            let tmp = p.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_string_pretty(&self.project)?)?;
            std::fs::rename(&tmp, p)?;
        }
        Ok(())
    }

    pub fn save_as(&mut self, path: &Path) -> Result<()> {
        self.path = Some(path.to_path_buf());
        self.media.base_dir = path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
        self.save()
    }

    /// Reload if the file on disk differs (used by the viewer to follow agent edits).
    pub fn reload_from_disk(&mut self) -> Result<bool> {
        let Some(p) = &self.path else { return Ok(false) };
        let text = std::fs::read_to_string(p)?;
        let project: Project = serde_json::from_str(&text)?;
        if project != self.project {
            self.project = project;
            self.refresh_library();
            return Ok(true);
        }
        Ok(false)
    }

    pub(crate) fn services(&self) -> Arc<dyn ScriptServices> {
        Arc::new(EngineServices { ff: self.media.ff.clone(), base_dir: self.base_dir(), cache_dir: self.cache_dir() })
    }

    fn refresh_library(&mut self) {
        let key = key_of((
            serde_json::to_string(&self.project.custom_effects).unwrap_or_default(),
            serde_json::to_string(&self.project.custom_presets).unwrap_or_default(),
        ));
        if key != self.library_key {
            self.library = Arc::new(Library::for_project(&self.project));
            self.library_key = key;
        }
        self.expr.set_library(&self.project.script_library);
    }

    pub fn library(&mut self) -> Arc<Library> {
        self.refresh_library();
        self.library.clone()
    }

    /// Apply a change as one undoable step. On error the project is left untouched.
    pub fn mutate<T>(&mut self, label: &str, f: impl FnOnce(&mut Project) -> std::result::Result<T, EditError>) -> Result<T> {
        let before = self.project.clone();
        match f(&mut self.project) {
            Ok(v) => {
                if self.project != before {
                    self.history.record(label, &before);
                    self.refresh_library();
                    if self.autosave {
                        self.save()?;
                    }
                }
                Ok(v)
            }
            Err(e) => {
                self.project = before;
                Err(e.into())
            }
        }
    }

    /// Replace the whole project as one undoable step.
    pub fn replace_project(&mut self, label: &str, project: Project) -> Result<bool> {
        if project == self.project {
            return Ok(false);
        }
        let before = std::mem::replace(&mut self.project, project);
        self.history.record(label, &before);
        self.refresh_library();
        if self.autosave {
            self.save()?;
        }
        Ok(true)
    }

    pub fn undo(&mut self) -> Result<Option<String>> {
        let r = self.history.undo(&mut self.project);
        self.refresh_library();
        if self.autosave {
            self.save()?;
        }
        Ok(r)
    }

    pub fn redo(&mut self) -> Result<Option<String>> {
        let r = self.history.redo(&mut self.project);
        self.refresh_library();
        if self.autosave {
            self.save()?;
        }
        Ok(r)
    }

    // ------------------------------------------------------------------------------------------
    // GPU
    // ------------------------------------------------------------------------------------------

    pub fn renderer(&mut self) -> Result<&mut Renderer> {
        if self.renderer.is_none() {
            let gpu = GpuContext::new(&self.gpu_options)?;
            self.renderer = Some(Renderer::new(gpu));
        }
        Ok(self.renderer.as_mut().unwrap())
    }

    /// Use an externally created device (viewer).
    pub fn set_renderer(&mut self, r: Renderer) {
        self.renderer = Some(r);
    }

    pub fn gpu_info(&mut self) -> Result<String> {
        Ok(self.renderer()?.gpu.describe())
    }

    // ------------------------------------------------------------------------------------------
    // Assets
    // ------------------------------------------------------------------------------------------

    fn store_path(&self, p: &Path) -> String {
        let abs = std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
        let base = std::fs::canonicalize(self.base_dir()).unwrap_or_else(|_| self.base_dir());
        match abs.strip_prefix(&base) {
            Ok(rel) if self.path.is_some() => rel.to_string_lossy().replace('\\', "/"),
            _ => abs.to_string_lossy().to_string(),
        }
    }

    fn expand_inputs(&self, inputs: &[String], opts: &ImportOptions) -> Vec<PathBuf> {
        let mut out = vec![];
        for inp in inputs {
            let p = resolve(&self.base_dir(), inp);
            if opts.kind == Some(AssetKind::ImageSequence) || inp.contains('%') {
                out.push(p);
                continue;
            }
            if inp.contains('*') || inp.contains('?') {
                if let Ok(paths) = glob::glob(&p.to_string_lossy()) {
                    out.extend(paths.flatten().filter(|x| x.is_file()));
                }
                continue;
            }
            if p.is_dir() {
                let mut stack = vec![p];
                while let Some(d) = stack.pop() {
                    if let Ok(rd) = std::fs::read_dir(&d) {
                        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
                        entries.sort();
                        for e in entries {
                            if e.is_dir() {
                                let hidden = e.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'));
                                if opts.recursive && !hidden {
                                    stack.push(e);
                                }
                            } else {
                                let name = e.file_name().and_then(|n| n.to_str()).unwrap_or("");
                                let skip = name.ends_with(".edits.json")
                                    || name.ends_with(".json.tmp")
                                    || name.starts_with('.')
                                    || name.contains(".preview.");
                                if !skip && edits_media::probe::guess_kind(&e.to_string_lossy()) != AssetKind::Auto {
                                    out.push(e);
                                }
                            }
                        }
                    }
                }
                continue;
            }
            out.push(p);
        }
        out
    }

    /// Import files, directories or globs as assets (probed in parallel).
    pub fn import(&mut self, inputs: &[String], opts: &ImportOptions) -> Result<Vec<ImportedAsset>> {
        use rayon::prelude::*;
        let paths = self.expand_inputs(inputs, opts);
        if paths.is_empty() {
            return Err(EngineError::NotFound(format!("no importable files in {inputs:?}")));
        }
        let ff = self.media.ff.clone();
        let kind = opts.kind.unwrap_or(AssetKind::Auto);
        let probed: Vec<(PathBuf, std::result::Result<(AssetKind, edits_core::MediaInfo), String>)> = paths
            .par_iter()
            .map(|p| (p.clone(), edits_media::probe::probe_asset(ff.as_ref(), p, kind).map_err(|e| e.to_string())))
            .collect();
        let mut errors = vec![];
        let mut new_assets = vec![];
        for (p, r) in probed {
            match r {
                Ok((k, mut info)) => {
                    if k == AssetKind::Font {
                        info.families = self.text.load_font_file(&p);
                        self.loaded_fonts.insert(p.clone());
                    }
                    let stored = self.store_path(&p);
                    if let Some((id, _)) = self.project.assets.iter().find(|(_, a)| a.path == stored) {
                        new_assets.push((id.clone(), None, k, stored, info));
                        continue;
                    }
                    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("asset");
                    let base = match (&opts.id, paths.len()) {
                        (Some(id), 1) => id.clone(),
                        _ => sanitize_id(stem),
                    };
                    new_assets.push((base, Some(p.clone()), k, stored, info));
                }
                Err(e) => errors.push(e),
            }
        }
        if new_assets.is_empty() {
            return Err(EngineError::Invalid(format!("import failed: {}", errors.join("; "))));
        }
        let tags = opts.tags.clone();
        let seq_fps = opts.sequence_fps;
        let mut out = vec![];
        self.mutate("import assets", |p| {
            for (base, fresh, k, stored, info) in new_assets {
                if fresh.is_none() {
                    // already imported: refresh info
                    if let Some(a) = p.assets.get_mut(&base) {
                        a.info = Some(info.clone());
                    }
                    out.push(ImportedAsset { id: base, kind: k, path: stored, info: Some(info) });
                    continue;
                }
                let mut id = base.clone();
                let mut n = 2;
                while p.id_exists(&id) {
                    id = format!("{base}_{n}");
                    n += 1;
                }
                p.assets.insert(
                    id.clone(),
                    Asset {
                        path: stored.clone(),
                        kind: k,
                        name: String::new(),
                        info: Some(info.clone()),
                        sequence_fps: if k == AssetKind::ImageSequence { Some(seq_fps.unwrap_or(24.0)) } else { None },
                        tags: tags.clone(),
                        notes: String::new(),
                    },
                );
                out.push(ImportedAsset { id, kind: k, path: stored, info: Some(info) });
            }
            Ok(())
        })?;
        if !errors.is_empty() {
            tracing::warn!("some imports failed: {}", errors.join("; "));
        }
        Ok(out)
    }

    fn ensure_fonts(&mut self) {
        let fonts: Vec<PathBuf> = self
            .project
            .assets
            .values()
            .filter(|a| a.kind == AssetKind::Font)
            .map(|a| self.media.resolve(&a.path))
            .filter(|p| !self.loaded_fonts.contains(p))
            .collect();
        for p in fonts {
            self.text.load_font_file(&p);
            self.loaded_fonts.insert(p);
        }
    }

    // ------------------------------------------------------------------------------------------
    // Analysis
    // ------------------------------------------------------------------------------------------

    /// Analyze music (cached on disk). With `apply`, writes beats/drops/sections into project timing.
    pub fn analyze_audio(
        &mut self,
        asset_id: &str,
        opts: &AnalysisOptions,
        apply: bool,
        offset: Option<f64>,
    ) -> Result<Arc<AudioAnalysis>> {
        let a = self.project.assets.get(asset_id).cloned().ok_or_else(|| EngineError::NotFound(format!("asset '{asset_id}'")))?;
        let an = self.load_or_analyze(asset_id, &a, opts)?;
        if apply {
            let offset = offset.unwrap_or_else(|| {
                // align to the first clip using this asset on the root comp
                let root = self.project.root.clone();
                self.project
                    .all_clips()
                    .filter(|(c, _, cl)| {
                        *c == root && matches!(&cl.source, edits_core::ClipSource::Media { asset, .. } if asset == asset_id)
                    })
                    .map(|(_, _, cl)| cl.start - cl.source_in)
                    .next()
                    .unwrap_or(0.0)
            });
            let an2 = an.clone();
            let id = asset_id.to_string();
            self.mutate("apply audio analysis", move |p| {
                p.timing = edits_core::Timing {
                    source: Some(id),
                    offset,
                    bpm: Some(an2.bpm),
                    beats: an2.beats.clone(),
                    downbeats: an2.downbeats.clone(),
                    drops: an2.drops.clone(),
                    accents: an2.accents.clone(),
                    sections: an2
                        .sections
                        .iter()
                        .map(|s| edits_core::Section { start: s.start, end: s.end, label: s.label.clone(), energy: s.energy })
                        .collect(),
                };
                Ok(())
            })?;
        }
        Ok(an)
    }

    fn load_or_analyze(&mut self, asset_id: &str, a: &Asset, opts: &AnalysisOptions) -> Result<Arc<AudioAnalysis>> {
        let p = self.media.resolve(&a.path);
        let key = file_key(&p, &serde_json::to_string(opts).unwrap_or_default());
        let mem_key = format!("{asset_id}:{key}");
        if let Some(an) = self.analyses.get(&mem_key) {
            return Ok(an.clone());
        }
        let cfile = self.cache_dir().join(format!("audio-{key}.json"));
        if let Ok(s) = std::fs::read_to_string(&cfile)
            && let Ok(an) = serde_json::from_str::<AudioAnalysis>(&s)
        {
            let an = Arc::new(an);
            self.analyses.insert(mem_key, an.clone());
            return Ok(an);
        }
        let buf = self.media.audio(a)?;
        let an = Arc::new(edits_audio::analyze(&buf, opts));
        let _ = std::fs::create_dir_all(self.cache_dir());
        let _ = std::fs::write(&cfile, serde_json::to_string(&*an).unwrap_or_default());
        self.analyses.insert(mem_key, an.clone());
        Ok(an)
    }

    /// Cached analysis for the timing source only (never computes).
    fn timing_envelope(&mut self) -> Option<Arc<edits_audio::Envelope>> {
        let src = self.project.timing.source.clone()?;
        let a = self.project.assets.get(&src)?.clone();
        let p = self.media.resolve(&a.path);
        let key = file_key(&p, &serde_json::to_string(&AnalysisOptions::default()).unwrap_or_default());
        let mem_key = format!("{src}:{key}");
        if let Some(an) = self.analyses.get(&mem_key) {
            return Some(Arc::new(an.envelope.clone()));
        }
        // any cached analysis of this file
        if let Some((_, an)) = self.analyses.iter().find(|(k, _)| k.starts_with(&format!("{src}:"))) {
            return Some(Arc::new(an.envelope.clone()));
        }
        let cfile = self.cache_dir().join(format!("audio-{key}.json"));
        let an: AudioAnalysis = serde_json::from_str(&std::fs::read_to_string(cfile).ok()?).ok()?;
        let an = Arc::new(an);
        self.analyses.insert(mem_key, an.clone());
        Some(Arc::new(an.envelope.clone()))
    }

    pub fn timing_data(&mut self) -> Arc<TimingData> {
        let env = self.timing_envelope();
        let t = &self.project.timing;
        let off = t.offset;
        Arc::new(TimingData {
            bpm: t.bpm.unwrap_or(0.0),
            beats: t.beats.iter().map(|b| b + off).collect(),
            downbeats: t.downbeats.iter().map(|b| b + off).collect(),
            drops: t.drops.iter().map(|b| b + off).collect(),
            accents: t.accents.iter().map(|b| b + off).collect(),
            envelope: env,
            envelope_offset: off,
        })
    }

    pub fn detect_scenes(&mut self, asset_id: &str, opts: &SceneOptions) -> Result<Arc<Vec<Shot>>> {
        let a = self.project.assets.get(asset_id).cloned().ok_or_else(|| EngineError::NotFound(format!("asset '{asset_id}'")))?;
        let k = format!("{asset_id}:{}", serde_json::to_string(opts).unwrap_or_default());
        if let Some(s) = self.scenes.get(&k) {
            return Ok(s.clone());
        }
        let s = detect_scenes_cached(self.media.ff.as_ref(), &self.base_dir(), &self.cache_dir(), &a, opts)?;
        self.scenes.insert(k, s.clone());
        Ok(s)
    }

    // ------------------------------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------------------------------

    /// Render one frame of a composition (default: root).
    pub fn render_frame(&mut self, comp: Option<&str>, t: f64, mode: OutputMode) -> Result<RenderedFrame> {
        let start = Instant::now();
        let comp_id = comp.map(String::from).unwrap_or_else(|| self.project.root.clone());
        if !self.project.compositions.contains_key(&comp_id) {
            return Err(EngineError::NotFound(format!("composition '{comp_id}'")));
        }
        self.ensure_fonts();
        let lib = self.library();
        let timing = self.timing_data();
        let vars = Arc::new(self.project.variables.clone());
        self.renderer()?;
        let Engine { renderer, media, project, text, expr, .. } = self;
        let renderer = renderer.as_mut().unwrap();
        let mut ctx = EvalCtx {
            project,
            lib: &lib,
            media,
            text,
            expr,
            timing,
            vars,
            warnings: RefCell::new(vec![]),
            expr_errors: Default::default(),
            root_time: t,
        };
        let mut f = renderer.frame();
        let out = ctx.render_comp(&mut f, &comp_id, t, 0)?;
        let frame = f.read(out, mode)?;
        let mut warnings = ctx.warnings.into_inner();
        warnings.extend(ctx.expr_errors.borrow().iter().cloned());
        Ok(RenderedFrame { frame, time: t, ms: start.elapsed().as_secs_f64() * 1000.0, warnings })
    }

    /// Render directly into a texture view (zero-copy display in the viewer). Returns warnings.
    pub fn render_to_view(
        &mut self,
        comp: Option<&str>,
        t: f64,
        view: &edits_render::wgpu::TextureView,
        format: edits_render::wgpu::TextureFormat,
        mode: OutputMode,
    ) -> Result<Vec<String>> {
        let comp_id = comp.map(String::from).unwrap_or_else(|| self.project.root.clone());
        if !self.project.compositions.contains_key(&comp_id) {
            return Err(EngineError::NotFound(format!("composition '{comp_id}'")));
        }
        self.ensure_fonts();
        let lib = self.library();
        let timing = self.timing_data();
        let vars = Arc::new(self.project.variables.clone());
        self.renderer()?;
        let Engine { renderer, media, project, text, expr, .. } = self;
        let renderer = renderer.as_mut().unwrap();
        let mut ctx = EvalCtx {
            project,
            lib: &lib,
            media,
            text,
            expr,
            timing,
            vars,
            warnings: RefCell::new(vec![]),
            expr_errors: Default::default(),
            root_time: t,
        };
        let mut f = renderer.frame();
        let out = ctx.render_comp(&mut f, &comp_id, t, 0)?;
        f.present_to(out, view, format, mode)?;
        let mut w = ctx.warnings.into_inner();
        w.extend(ctx.expr_errors.borrow().iter().cloned());
        Ok(w)
    }

    /// Render several times and tile them into a labeled contact sheet.
    pub fn contact_sheet(&mut self, comp: Option<&str>, times: &[f64], columns: u32, thumb_width: u32) -> Result<(Frame, Vec<String>)> {
        let mut thumbs = vec![];
        let mut warnings = vec![];
        for t in times {
            let r = self.render_frame(comp, *t, OutputMode::Over([0.0, 0.0, 0.0, 1.0]))?;
            for w in r.warnings {
                if !warnings.contains(&w) {
                    warnings.push(w);
                }
            }
            thumbs.push((*t, r.frame));
        }
        let labels: Vec<String> = times.iter().map(|t| format!("{t:.2}s")).collect();
        Ok((self.tile(&thumbs.into_iter().map(|x| x.1).collect::<Vec<_>>(), &labels, columns, thumb_width), warnings))
    }

    /// Tile frames into a grid with small text labels.
    pub fn tile(&self, frames: &[Frame], labels: &[String], columns: u32, thumb_width: u32) -> Frame {
        let cols = columns.clamp(1, 16) as usize;
        let tw = thumb_width.clamp(32, 1920);
        let th = frames.first().map(|f| (tw as f64 * f.height as f64 / f.width.max(1) as f64).round() as u32).unwrap_or(tw * 9 / 16).max(8);
        let rows = frames.len().div_ceil(cols).max(1);
        let gap = 4u32;
        let (w, h) = (cols as u32 * (tw + gap) + gap, rows as u32 * (th + gap) + gap);
        let mut sheet = image::RgbaImage::from_pixel(w, h, image::Rgba([24, 24, 28, 255]));
        let label_src = |s: &str| {
            let mut t = edits_core::TextSource::simple(s, (th as f64 / 9.0).clamp(10.0, 28.0));
            t.background = Some(edits_core::TextBackground {
                color: edits_core::Color([0.0, 0.0, 0.0, 0.65]),
                padding: edits_core::Vec2::new(6.0, 2.0),
                radius: 3.0,
            });
            t.weight = 600;
            t
        };
        for (i, f) in frames.iter().enumerate() {
            let Some(img) = image::RgbaImage::from_raw(f.width, f.height, f.data.clone()) else { continue };
            let small = image::imageops::resize(&img, tw, th, image::imageops::FilterType::Triangle);
            let (x, y) = (gap + (i % cols) as u32 * (tw + gap), gap + (i / cols) as u32 * (th + gap));
            image::imageops::overlay(&mut sheet, &small, x as i64, y as i64);
            if let Some(l) = labels.get(i) {
                let lf = self.text.render(
                    &label_src(l),
                    &edits_render::TextFrameParams { color: edits_core::Color::WHITE, letter_spacing: 0.0, time: 0.0, duration: 1.0 },
                );
                if let Some(limg) = image::RgbaImage::from_raw(lf.width, lf.height, lf.data) {
                    image::imageops::overlay(&mut sheet, &limg, x as i64 - 4, y as i64 - 4);
                }
            }
        }
        Frame::new(w, h, sheet.into_raw(), false)
    }

    /// Thumbnails straight from a video asset (no project rendering), e.g. for shot browsing.
    pub fn asset_sheet(&mut self, asset_id: &str, times: &[f64], columns: u32, thumb_width: u32) -> Result<Frame> {
        let a = self.project.assets.get(asset_id).cloned().ok_or_else(|| EngineError::NotFound(format!("asset '{asset_id}'")))?;
        let info = a.info.clone().unwrap_or_default();
        let th = (thumb_width as f64 * info.height.max(1) as f64 / info.width.max(1) as f64).round() as u32;
        let mut frames = vec![];
        for t in times {
            let f = match a.kind {
                AssetKind::Video => {
                    edits_media::video::grab_frame(self.media.ffmpeg()?, &self.media.resolve(&a.path), *t, thumb_width, th.max(2))?
                }
                _ => (*self.media.visual(&a, *t, (thumb_width, th), None)?.frame).clone(),
            };
            frames.push(f);
        }
        let labels: Vec<String> = times.iter().map(|t| format!("{t:.2}s")).collect();
        Ok(self.tile(&frames, &labels, columns, thumb_width))
    }

    /// Waveform with beat grid / downbeats / drops / sections overlay.
    pub fn waveform_image(&mut self, asset_id: &str, t0: f64, t1: Option<f64>, width: u32, height: u32) -> Result<Frame> {
        let a = self.project.assets.get(asset_id).cloned().ok_or_else(|| EngineError::NotFound(format!("asset '{asset_id}'")))?;
        let buf = self.media.audio(&a)?;
        let t1 = t1.unwrap_or(buf.duration()).max(t0 + 0.01);
        let (w, h) = (width.clamp(64, 4096), height.clamp(32, 2048));
        let cols = edits_audio::waveform::columns(&buf, t0, t1, w as usize);
        let mut img = image::RgbaImage::from_pixel(w, h, image::Rgba([16, 16, 22, 255]));
        let x_of = |t: f64| (((t - t0) / (t1 - t0)) * w as f64) as i64;
        // sections band
        let timing = self.project.timing.clone();
        let off = if timing.source.as_deref() == Some(asset_id) { 0.0 } else { f64::NAN };
        if !off.is_nan() {
            for s in &timing.sections {
                let c = match s.label.as_str() {
                    "drop" | "chorus" => [120, 30, 60, 255],
                    "build" => [110, 80, 20, 255],
                    "intro" | "outro" => [30, 40, 80, 255],
                    _ => [40, 60, 50, 255],
                };
                for x in x_of(s.start).max(0)..x_of(s.end).min(w as i64) {
                    for y in 0..(h / 12).max(3) {
                        img.put_pixel(x as u32, y, image::Rgba(c));
                    }
                }
            }
        }
        let mid = h as f64 / 2.0;
        for (x, (mn, mx, rms)) in cols.iter().enumerate() {
            let y0 = (mid - *mx as f64 * mid * 0.9).clamp(0.0, h as f64 - 1.0) as u32;
            let y1 = (mid - *mn as f64 * mid * 0.9).clamp(0.0, h as f64 - 1.0) as u32;
            for y in y0..=y1 {
                img.put_pixel(x as u32, y, image::Rgba([90, 140, 220, 255]));
            }
            let r0 = (mid - *rms as f64 * mid * 0.9) as u32;
            let r1 = (mid + *rms as f64 * mid * 0.9).min(h as f64 - 1.0) as u32;
            for y in r0..=r1 {
                img.put_pixel(x as u32, y, image::Rgba([150, 200, 255, 255]));
            }
        }
        if !off.is_nan() {
            let mut vline = |t: f64, c: [u8; 4], every: u32| {
                let x = x_of(t);
                if x >= 0 && x < w as i64 {
                    for y in (0..h).step_by(every as usize) {
                        img.put_pixel(x as u32, y, image::Rgba(c));
                    }
                }
            };
            for b in &timing.beats {
                vline(*b, [90, 90, 100, 255], 3);
            }
            for b in &timing.downbeats {
                vline(*b, [230, 230, 230, 255], 1);
            }
            for d in &timing.drops {
                vline(*d, [255, 60, 90, 255], 1);
            }
        }
        Ok(Frame::new(w, h, img.into_raw(), false))
    }

    /// Render an effect on a frame of the edit (or a test card if the project is empty), so an
    /// agent can see what it does before using it. Transitions are shown at `progress`.
    pub fn preview_effect(
        &mut self,
        effect: &str,
        params: &serde_json::Map<String, Json>,
        time: Option<f64>,
        progress: f64,
    ) -> Result<Frame> {
        let lib = self.library();
        let def = lib.effect(effect).cloned().ok_or_else(|| EngineError::NotFound(format!("effect '{effect}'")))?;
        let comp = self.project.root_comp().cloned().ok_or_else(|| EngineError::NotFound("root composition".into()))?;
        let (w, h) = (comp.width, comp.height);
        let has_content = comp.tracks.iter().any(|t| !t.clips.is_empty());
        let t = time.unwrap_or(comp.duration / 2.0);
        let base = if has_content { Some(self.render_frame(None, t, OutputMode::Over([0.0, 0.0, 0.0, 1.0]))?.frame) } else { None };
        let card = test_card(w, h);
        let second = test_card_alt(w, h);
        let props: indexmap::IndexMap<String, edits_core::Property<edits_core::Value>> =
            serde_json::from_value(Json::Object(params.clone())).map_err(|e| EngineError::Invalid(format!("bad params: {e}")))?;
        for k in props.keys() {
            if def.param(k).is_none() {
                return Err(EngineError::Invalid(format!("effect '{effect}' has no param '{k}'")));
            }
        }
        let slots: Vec<[f32; 4]> = def
            .slot_params()
            .map(|p| {
                let v = props.get(&p.name).map(|pr| pr.sample(0.0, &p.default)).unwrap_or_else(|| p.default.clone());
                p.to_slot(&v)
            })
            .collect();
        let timing = self.timing_data();
        let r = self.renderer()?;
        let mut f = r.frame();
        let a = f.upload(None, base.as_ref().unwrap_or(&card));
        let b = f.upload(None, &second);
        let g = edits_render::Globals {
            time: t as f32,
            local_time: t as f32,
            progress: progress as f32,
            duration: 2.0,
            bpm: timing.bpm as f32,
            beat_time: 0.05,
            seed: 0.37,
            ..Default::default()
        };
        let (input, input2) = match def.kind {
            edits_fx::EffectKind::Filter => (Some(a), None),
            edits_fx::EffectKind::Transition => (Some(a), Some(b)),
            edits_fx::EffectKind::Generator => (None, None),
        };
        let out = f.effect(edits_render::EffectCall { def: &def, params: &slots, globals: g, input, input2, extra: None, size: (w, h) })?;
        Ok(f.read(out, OutputMode::Over([0.0, 0.0, 0.0, 1.0]))?)
    }

    // ------------------------------------------------------------------------------------------
    // Presets & scripts
    // ------------------------------------------------------------------------------------------

    pub fn apply_preset(
        &mut self,
        preset: &str,
        target: Option<&str>,
        args: &serde_json::Map<String, Json>,
        at: Option<f64>,
    ) -> Result<ScriptReport> {
        let lib = self.library();
        let def = lib.preset(preset).cloned().ok_or_else(|| EngineError::NotFound(format!("preset '{preset}' (see presets_list)")))?;
        let out = script::run_preset(self.project.clone(), lib, self.services(), &def, target, args, at, 0, 0)?;
        let changed = self.replace_project(&format!("preset {preset}"), out.project)?;
        Ok(ScriptReport { logs: out.logs, result: out.result, changed })
    }

    pub fn run_script(&mut self, code: &str, args: Json, dry_run: bool) -> Result<(ScriptReport, Option<Project>)> {
        let lib = self.library();
        let out = script::run_script(self.project.clone(), lib, self.services(), code, args)?;
        let problems: Vec<String> = edits_core::validate::validate(&out.project)
            .into_iter()
            .filter(|i| i.severity == edits_core::validate::Severity::Error)
            .map(|i| format!("{}: {}", i.object.unwrap_or_default(), i.message))
            .collect();
        if !problems.is_empty() {
            return Err(EngineError::Invalid(format!("script result has errors (not applied): {}", problems.join("; "))));
        }
        if dry_run {
            let changed = out.project != self.project;
            return Ok((ScriptReport { logs: out.logs, result: out.result, changed }, Some(out.project)));
        }
        let changed = self.replace_project("script", out.project)?;
        Ok((ScriptReport { logs: out.logs, result: out.result, changed }, None))
    }

    // ------------------------------------------------------------------------------------------
    // Validation
    // ------------------------------------------------------------------------------------------

    pub fn validate(&mut self) -> Vec<Issue> {
        use edits_core::validate::Severity;
        let mut issues = edits_core::validate::validate(&self.project);
        let lib = self.library();
        let push = |issues: &mut Vec<Issue>, sev: Severity, obj: Option<&str>, msg: String| {
            issues.push(Issue { severity: sev, object: obj.map(String::from), message: msg });
        };
        for (id, e) in &lib.errors {
            push(&mut issues, Severity::Error, Some(id), format!("custom definition error: {e}"));
        }
        for (id, a) in &self.project.assets {
            let p = self.media.resolve(&a.path);
            let exists = if a.kind == AssetKind::ImageSequence { p.parent().is_some_and(|d| d.exists()) || p.exists() } else { p.exists() };
            if !exists {
                push(&mut issues, Severity::Error, Some(id), format!("file not found: {}", p.display()));
            }
        }
        let check_fx = |issues: &mut Vec<Issue>, list: &[edits_core::EffectInstance], owner: &str| {
            for e in list {
                match lib.effect(&e.effect) {
                    None => issues.push(Issue {
                        severity: Severity::Error,
                        object: Some(owner.into()),
                        message: format!("unknown effect '{}'", e.effect),
                    }),
                    Some(d) => {
                        if d.kind != edits_fx::EffectKind::Filter {
                            issues.push(Issue {
                                severity: Severity::Error,
                                object: Some(owner.into()),
                                message: format!("'{}' is a {}, not a filter", d.id, d.kind.as_str()),
                            });
                        }
                        for k in e.params.keys() {
                            if d.param(k).is_none() {
                                issues.push(Issue {
                                    severity: Severity::Warning,
                                    object: Some(e.id.clone()),
                                    message: format!("effect '{}' has no param '{k}'", d.id),
                                });
                            }
                        }
                    }
                }
            }
        };
        for (cid, comp) in &self.project.compositions {
            check_fx(&mut issues, &comp.effects, cid);
            for t in &comp.tracks {
                check_fx(&mut issues, &t.effects, &t.id);
                for c in &t.clips {
                    check_fx(&mut issues, &c.effects, &c.id);
                    if let Some(tr) = &c.transition_in
                        && lib.effect(&tr.effect).map(|d| d.kind) != Some(edits_fx::EffectKind::Transition)
                    {
                        push(&mut issues, Severity::Error, Some(&c.id), format!("unknown transition '{}'", tr.effect));
                    }
                    if let edits_core::ClipSource::Generator { effect, .. } = &c.source
                        && lib.effect(effect).map(|d| d.kind) != Some(edits_fx::EffectKind::Generator)
                    {
                        push(&mut issues, Severity::Error, Some(&c.id), format!("unknown generator '{effect}'"));
                    }
                }
            }
        }
        // compile every expression
        let v = serde_json::to_value(&self.project).unwrap_or_default();
        let mut exprs = vec![];
        collect_exprs(&v, &mut exprs);
        for e in exprs {
            if let Err(err) = self.expr.compile(&e) {
                push(&mut issues, Severity::Error, None, format!("expression `{e}` does not compile: {err}"));
            }
        }
        issues.sort_by(|a, b| b.severity.cmp(&a.severity));
        issues
    }

    pub fn status(&mut self) -> Json {
        json!({
            "project": self.project.meta.name,
            "path": self.path.as_ref().map(|p| p.display().to_string()),
            "revision": self.history.revision,
            "can_undo": self.history.can_undo(),
            "can_redo": self.history.can_redo(),
            "ffmpeg": self.media.ff.as_ref().map(|f| f.version.clone()),
            "effects": self.library.effect_count(),
            "presets": self.library.preset_count(),
        })
    }
}

/// Colorful test card used when there is nothing to preview.
pub fn test_card(w: u32, h: u32) -> Frame {
    let mut d = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            let cx = u - 0.5;
            let cy = (v - 0.5) * h as f32 / w as f32;
            let r = (cx * cx + cy * cy).sqrt();
            let ring = ((r * 40.0).sin() * 0.5 + 0.5) * (1.0 - (r * 2.2).min(1.0));
            let checker = (((x / 32) + (y / 32)) % 2) as f32;
            let col = [
                (0.5 + 0.5 * (u * std::f32::consts::TAU).sin()) * 0.8 + ring * 0.6,
                (0.5 + 0.5 * (v * std::f32::consts::TAU + 2.0).sin()) * 0.7 + checker * 0.1,
                (0.5 + 0.5 * ((u + v) * std::f32::consts::TAU + 4.0).sin()) * 0.9 + ring * 0.3,
            ];
            d.extend(col.iter().map(|c| (c.clamp(0.0, 1.0) * 255.0) as u8));
            d.push(255);
        }
    }
    Frame::new(w, h, d, false)
}

fn test_card_alt(w: u32, h: u32) -> Frame {
    let mut f = test_card(w, h);
    for px in f.data.chunks_exact_mut(4) {
        px.swap(0, 2);
        px[1] = 255 - px[1];
    }
    f
}

fn collect_exprs(v: &Json, out: &mut Vec<String>) {
    match v {
        Json::Object(m) => {
            if let Some(Json::String(e)) = m.get("expr") {
                out.push(e.clone());
            }
            for c in m.values() {
                collect_exprs(c, out);
            }
        }
        Json::Array(a) => a.iter().for_each(|c| collect_exprs(c, out)),
        _ => {}
    }
}
