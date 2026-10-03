//! EditsEditor viewer: a live window onto a project file. It follows the file as AI agents edit
//! it (via MCP/CLI), renders on the same GPU device as the UI (zero-copy), and offers scrubbing,
//! playback with audio (Windows), a timeline with beat/drop markers, an inspector with JSON
//! editing, effect/preset browsers and undo/redo.

mod audio;

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use edits_core::{ClipSource, ops, summary};
use edits_engine::{Engine, OutputMode};
use edits_render::{GpuContext, Renderer, wgpu};
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};

const PREVIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

struct Preview {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    id: egui::TextureId,
    size: (u32, u32),
}

#[derive(PartialEq)]
enum SideTab {
    Inspector,
    Effects,
    Presets,
    Project,
}

struct App {
    engine: Option<Engine>,
    path: Option<PathBuf>,
    path_input: String,
    last_mtime: Option<SystemTime>,
    last_check: Instant,
    revision: u64,
    time: f64,
    playing: bool,
    play_started: Instant,
    play_origin: f64,
    looping: bool,
    checker: bool,
    preview: Option<Preview>,
    rendered: Option<(u64, f64)>,
    render_ms: f64,
    warnings: Vec<String>,
    status: String,
    selected: Option<String>,
    inspector_text: String,
    inspector_for: Option<(String, u64)>,
    tab: SideTab,
    search: String,
    px_per_sec: f32,
    scroll_x: f32,
    audio: audio::AudioPlayer,
    audio_rev: u64,
    rs: egui_wgpu_state::State,
    gpu: Option<GpuContext>,
}

/// Small wrapper so we don't depend on egui_wgpu directly by name.
mod egui_wgpu_state {
    pub struct State {
        pub device: edits_render::wgpu::Device,
        pub renderer: std::sync::Arc<egui::mutex::RwLock<eframe::egui_wgpu::Renderer>>,
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, path: Option<PathBuf>) -> App {
        let rs = cc.wgpu_render_state.as_ref().expect("wgpu backend required");
        let mut app = App {
            engine: None,
            path: None,
            path_input: path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            last_mtime: None,
            last_check: Instant::now(),
            revision: 0,
            time: 0.0,
            playing: false,
            play_started: Instant::now(),
            play_origin: 0.0,
            looping: true,
            checker: false,
            preview: None,
            rendered: None,
            render_ms: 0.0,
            warnings: vec![],
            status: "open a project (File → path) or run `edits view project.edits.json`".into(),
            selected: None,
            inspector_text: String::new(),
            inspector_for: None,
            tab: SideTab::Inspector,
            search: String::new(),
            px_per_sec: 80.0,
            scroll_x: 0.0,
            audio: audio::AudioPlayer::new(),
            audio_rev: u64::MAX,
            rs: egui_wgpu_state::State { device: rs.device.clone(), renderer: rs.renderer.clone() },
            gpu: None,
        };
        let gpu = GpuContext::from_device(rs.device.clone(), rs.queue.clone(), rs.adapter.get_info());
        app.gpu = Some(gpu);
        if let Some(p) = path {
            app.open(p);
        }
        app
    }

    fn open(&mut self, p: PathBuf) {
        match Engine::open(&p) {
            Ok(mut e) => {
                if let Some(g) = self.gpu.clone() {
                    e.set_renderer(Renderer::new(g));
                }
                e.autosave = true;
                self.last_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
                self.status = format!("opened {}", p.display());
                self.path = Some(p);
                self.engine = Some(e);
                self.revision += 1;
                self.rendered = None;
            }
            Err(err) => self.status = format!("open failed: {err}"),
        }
    }

    fn duration(&self) -> f64 {
        self.engine.as_ref().and_then(|e| e.project.root_comp()).map(|c| c.duration).unwrap_or(10.0)
    }

    fn fps(&self) -> f64 {
        self.engine.as_ref().and_then(|e| e.project.root_comp()).map(|c| c.fps).unwrap_or(24.0)
    }

    /// Follow external edits (agents writing the project file).
    fn poll_file(&mut self) {
        if self.last_check.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.last_check = Instant::now();
        let (Some(p), Some(e)) = (&self.path, self.engine.as_mut()) else { return };
        let mtime = std::fs::metadata(p).and_then(|m| m.modified()).ok();
        if mtime != self.last_mtime {
            self.last_mtime = mtime;
            match e.reload_from_disk() {
                Ok(true) => {
                    self.revision += 1;
                    self.status = format!("reloaded (agent edit) at {}", chrono_now());
                }
                Ok(false) => {}
                Err(err) => self.status = format!("reload failed (file mid-write?): {err}"),
            }
        }
    }

    fn after_local_edit(&mut self) {
        self.revision += 1;
        if let Some(p) = &self.path {
            self.last_mtime = std::fs::metadata(p).and_then(|m| m.modified()).ok();
        }
    }

    fn ensure_preview(&mut self, w: u32, h: u32) {
        if self.preview.as_ref().is_some_and(|p| p.size == (w, h)) {
            return;
        }
        let texture = self.rs.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("viewer-preview"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PREVIEW_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut r = self.rs.renderer.write();
        let id = match &self.preview {
            Some(p) => {
                r.update_egui_texture_from_wgpu_texture(&self.rs.device, &view, wgpu::FilterMode::Linear, p.id);
                p.id
            }
            None => r.register_native_texture(&self.rs.device, &view, wgpu::FilterMode::Linear),
        };
        drop(r);
        self.preview = Some(Preview { _texture: texture, view, id, size: (w, h) });
        self.rendered = None;
    }

    fn render_if_needed(&mut self) {
        let Some(e) = self.engine.as_ref() else { return };
        let Some(c) = e.project.root_comp() else { return };
        let (w, h) = (c.width, c.height);
        self.ensure_preview(w, h);
        let key = (self.revision, (self.time * self.fps()).floor());
        if self.rendered == Some(key) {
            return;
        }
        let start = Instant::now();
        let mode = if self.checker { OutputMode::Checker } else { OutputMode::Over([0.0, 0.0, 0.0, 1.0]) };
        let p = self.preview.as_ref().unwrap();
        let view = p.view.clone();
        let e = self.engine.as_mut().unwrap();
        match e.render_to_view(None, self.time, &view, PREVIEW_FORMAT, mode) {
            Ok(w) => self.warnings = w,
            Err(err) => self.warnings = vec![format!("render failed: {err}")],
        }
        self.render_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.rendered = Some(key);
    }

    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        if self.playing {
            if self.time >= self.duration() - 1e-3 {
                self.time = 0.0;
            }
            self.play_started = Instant::now();
            self.play_origin = self.time;
            self.prepare_audio();
            self.audio.play(self.time);
        } else {
            self.audio.stop();
        }
    }

    fn prepare_audio(&mut self) {
        if !self.audio.enabled() || self.audio_rev == self.revision {
            return;
        }
        if let Some(e) = self.engine.as_mut() {
            let d = e.project.root_comp().map(|c| c.duration).unwrap_or(0.0);
            match e.mix_audio(None, 0.0, d) {
                Ok(m) => self.audio.set_mix(Some(Arc::new(m))),
                Err(err) => self.status = format!("audio mix failed: {err}"),
            }
        }
        self.audio_rev = self.revision;
    }

    fn advance(&mut self) {
        if !self.playing {
            return;
        }
        let t = if self.audio.enabled() && self.audio.has_mix() {
            self.audio.time()
        } else {
            self.play_origin + self.play_started.elapsed().as_secs_f64()
        };
        if t >= self.duration() {
            if self.looping {
                self.time = 0.0;
                self.play_started = Instant::now();
                self.play_origin = 0.0;
                self.audio.play(0.0);
            } else {
                self.time = self.duration();
                self.playing = false;
                self.audio.stop();
            }
        } else {
            self.time = t;
        }
    }

    fn seek(&mut self, t: f64) {
        self.time = t.clamp(0.0, self.duration());
        if self.playing {
            self.play_started = Instant::now();
            self.play_origin = self.time;
            self.audio.play(self.time);
        }
    }

    // ---------------------------------------------------------------- UI pieces

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("EditsEditor");
            ui.separator();
            ui.add(egui::TextEdit::singleline(&mut self.path_input).hint_text("project.edits.json").desired_width(320.0));
            if ui.button("Open").clicked() {
                let p = PathBuf::from(self.path_input.trim());
                self.open(p);
            }
            #[cfg(windows)]
            if ui.button("…").clicked()
                && let Some(p) = rfd::FileDialog::new().add_filter("EditsEditor project", &["json"]).pick_file()
            {
                self.path_input = p.display().to_string();
                self.open(p);
            }
            ui.separator();
            let can_undo = self.engine.as_ref().is_some_and(|e| e.history.can_undo());
            let can_redo = self.engine.as_ref().is_some_and(|e| e.history.can_redo());
            if ui.add_enabled(can_undo, egui::Button::new("⟲ Undo")).clicked() {
                if let Some(e) = self.engine.as_mut() {
                    let _ = e.undo();
                }
                self.after_local_edit();
            }
            if ui.add_enabled(can_redo, egui::Button::new("⟳ Redo")).clicked() {
                if let Some(e) = self.engine.as_mut() {
                    let _ = e.redo();
                }
                self.after_local_edit();
            }
            ui.separator();
            ui.checkbox(&mut self.checker, "transparency");
            ui.checkbox(&mut self.looping, "loop");
            ui.separator();
            ui.label(format!("{:.1} ms/frame", self.render_ms));
            if !self.audio.enabled() {
                ui.weak("(no audio playback on this platform)");
            }
        });
    }

    fn transport(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(if self.playing { "⏸" } else { "▶" }).clicked() {
                self.toggle_play();
            }
            let fps = self.fps();
            if ui.button("⏮").clicked() {
                self.seek(0.0);
            }
            if ui.button("◀ frame").clicked() {
                self.seek(self.time - 1.0 / fps);
            }
            if ui.button("frame ▶").clicked() {
                self.seek(self.time + 1.0 / fps);
            }
            if ui.button("prev beat").clicked() {
                let now = self.time;
                let prev = self.engine.as_ref().and_then(|e| e.project.timing.beats_abs().filter(|b| *b < now - 1e-3).last());
                if let Some(b) = prev {
                    self.seek(b);
                }
            }
            if ui.button("next beat").clicked() {
                let now = self.time;
                let next = self.engine.as_ref().and_then(|e| e.project.timing.beats_abs().find(|b| *b > now + 1e-3));
                if let Some(b) = next {
                    self.seek(b);
                }
            }
            let d = self.duration();
            let mut t = self.time;
            let w = ui.available_width() - 160.0;
            if ui.add_sized([w.max(100.0), 18.0], egui::Slider::new(&mut t, 0.0..=d).show_value(false)).changed() {
                self.seek(t);
            }
            let frame = (self.time * fps).floor() as u64;
            ui.monospace(format!("{:>7.3}s  f{frame}", self.time));
        });
    }

    fn timeline(&mut self, ui: &mut egui::Ui) {
        let Some(e) = self.engine.as_ref() else {
            ui.label("no project");
            return;
        };
        let Some(comp) = e.project.root_comp().cloned() else { return };
        let timing = e.project.timing.clone();
        let row_h = 26.0;
        let ruler_h = 22.0;
        let n_tracks = comp.tracks.len();
        let height = ruler_h + n_tracks as f32 * row_h + 8.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height.max(80.0)), Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 20, 26));
        // zoom & scroll
        if resp.hovered() {
            let (scroll, zoom) = ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
            if zoom != 1.0 {
                self.px_per_sec = (self.px_per_sec * zoom).clamp(5.0, 2000.0);
            }
            if ui.input(|i| i.modifiers.ctrl) && scroll.y != 0.0 {
                self.px_per_sec = (self.px_per_sec * (1.0 + scroll.y * 0.002)).clamp(5.0, 2000.0);
            } else {
                self.scroll_x = (self.scroll_x - scroll.x - scroll.y).max(0.0);
            }
        }
        let x_of = |t: f64| rect.left() + 8.0 + (t as f32 * self.px_per_sec) - self.scroll_x;
        let t_of = |x: f32| ((x - rect.left() - 8.0 + self.scroll_x) / self.px_per_sec) as f64;
        // ruler
        let ruler = Rect::from_min_size(rect.min, Vec2::new(rect.width(), ruler_h));
        painter.rect_filled(ruler, 0.0, Color32::from_rgb(32, 32, 40));
        let step = if self.px_per_sec > 200.0 {
            0.25
        } else if self.px_per_sec > 60.0 {
            1.0
        } else if self.px_per_sec > 20.0 {
            5.0
        } else {
            10.0
        };
        let mut t = 0.0;
        while t <= comp.duration {
            let x = x_of(t);
            if x > rect.right() {
                break;
            }
            painter.line_segment([Pos2::new(x, ruler.bottom() - 6.0), Pos2::new(x, ruler.bottom())], Stroke::new(1.0, Color32::GRAY));
            painter.text(
                Pos2::new(x + 2.0, ruler.top() + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{t:.0}"),
                egui::FontId::monospace(10.0),
                Color32::GRAY,
            );
            t += step;
        }
        // sections band & beats
        for s in &timing.sections {
            let c = match s.label.as_str() {
                "drop" | "chorus" => Color32::from_rgba_unmultiplied(200, 40, 90, 60),
                "build" => Color32::from_rgba_unmultiplied(200, 150, 30, 50),
                _ => Color32::from_rgba_unmultiplied(60, 90, 160, 40),
            };
            let r = Rect::from_x_y_ranges(x_of(s.start + timing.offset)..=x_of(s.end + timing.offset), ruler.top()..=ruler.bottom());
            painter.rect_filled(r, 0.0, c);
        }
        for b in timing.beats_abs() {
            let x = x_of(b);
            painter.line_segment(
                [Pos2::new(x, ruler.bottom()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 18)),
            );
        }
        for b in timing.downbeats.iter().map(|b| b + timing.offset) {
            let x = x_of(b);
            painter.line_segment(
                [Pos2::new(x, ruler.bottom() - 10.0), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 45)),
            );
        }
        for d in timing.drops.iter().map(|b| b + timing.offset) {
            let x = x_of(d);
            painter
                .line_segment([Pos2::new(x, ruler.top()), Pos2::new(x, rect.bottom())], Stroke::new(2.0, Color32::from_rgb(255, 60, 90)));
        }
        for m in &comp.markers {
            let x = x_of(m.t);
            painter.text(
                Pos2::new(x + 2.0, ruler.top() + 10.0),
                egui::Align2::LEFT_TOP,
                &m.label,
                egui::FontId::proportional(10.0),
                Color32::YELLOW,
            );
        }
        // tracks (top track drawn first)
        let mut clicked_clip = None;
        for (vi, (ti, track)) in comp.tracks.iter().enumerate().rev().enumerate() {
            let _ = ti;
            let y = ruler.bottom() + 4.0 + vi as f32 * row_h;
            painter.text(
                Pos2::new(rect.left() + 2.0, y + 2.0),
                egui::Align2::LEFT_TOP,
                &track.id,
                egui::FontId::monospace(9.0),
                Color32::from_gray(110),
            );
            for c in &track.clips {
                let r = Rect::from_x_y_ranges(x_of(c.start)..=x_of(c.end()).max(x_of(c.start) + 2.0), (y + 2.0)..=(y + row_h - 2.0));
                if r.right() < rect.left() || r.left() > rect.right() {
                    continue;
                }
                let base = match &c.source {
                    ClipSource::Media { asset, .. } => match e.project.assets.get(asset).map(|a| a.kind) {
                        Some(edits_core::AssetKind::Audio) => Color32::from_rgb(40, 120, 90),
                        Some(edits_core::AssetKind::Video) => Color32::from_rgb(50, 90, 170),
                        _ => Color32::from_rgb(110, 80, 160),
                    },
                    ClipSource::Text(_) => Color32::from_rgb(170, 110, 40),
                    ClipSource::Shape(_) => Color32::from_rgb(150, 60, 120),
                    ClipSource::Generator { .. } => Color32::from_rgb(140, 40, 160),
                    ClipSource::Comp { .. } => Color32::from_rgb(60, 140, 150),
                    ClipSource::Adjustment => Color32::from_rgb(90, 90, 90),
                    ClipSource::Solid { .. } => Color32::from_rgb(120, 120, 60),
                };
                let sel = self.selected.as_deref() == Some(c.id.as_str());
                let col = if c.enabled { base } else { base.gamma_multiply(0.4) };
                painter.rect_filled(r, 3.0, col);
                if sel {
                    painter.rect_stroke(r, 3.0, Stroke::new(2.0, Color32::WHITE), egui::StrokeKind::Inside);
                }
                if c.transition_in.is_some() {
                    painter.line_segment([r.left_top(), r.left_bottom()], Stroke::new(3.0, Color32::from_rgb(255, 220, 90)));
                }
                let label = match &c.source {
                    ClipSource::Text(t) => format!("{} “{}”", c.id, t.text.chars().take(20).collect::<String>()),
                    ClipSource::Media { asset, .. } => format!("{} {asset}", c.id),
                    ClipSource::Generator { effect, .. } => format!("{} {effect}", c.id),
                    other => format!("{} {}", c.id, other.kind_name()),
                };
                let fx = if c.effects.is_empty() { String::new() } else { format!(" ·{}fx", c.effects.len()) };
                painter.with_clip_rect(r).text(
                    r.left_center() + Vec2::new(4.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{label}{fx}"),
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );
                if resp.clicked() && resp.interact_pointer_pos().is_some_and(|p| r.contains(p)) {
                    clicked_clip = Some(c.id.clone());
                }
            }
        }
        // playhead
        let px = x_of(self.time);
        painter.line_segment([Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())], Stroke::new(2.0, Color32::from_rgb(255, 80, 80)));
        if let Some(id) = clicked_clip {
            self.selected = Some(id);
            self.tab = SideTab::Inspector;
        } else if (resp.clicked() || resp.dragged())
            && resp.interact_pointer_pos().is_some_and(|p| p.y < ruler.bottom() + 4.0 || resp.dragged())
            && let Some(p) = resp.interact_pointer_pos()
        {
            self.seek(t_of(p.x));
        }
    }

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.tab, SideTab::Inspector, "Inspector");
            ui.selectable_value(&mut self.tab, SideTab::Effects, "Effects");
            ui.selectable_value(&mut self.tab, SideTab::Presets, "Presets");
            ui.selectable_value(&mut self.tab, SideTab::Project, "Project");
        });
        ui.separator();
        match self.tab {
            SideTab::Inspector => self.inspector(ui),
            SideTab::Effects => self.library(ui, true),
            SideTab::Presets => self.library(ui, false),
            SideTab::Project => {
                if let Some(e) = self.engine.as_ref() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.monospace(summary::summarize(&e.project));
                        let (u, r) = e.history.labels();
                        ui.separator();
                        ui.label(format!("history: {} undo / {} redo", u.len(), r.len()));
                        for l in u.iter().rev().take(20) {
                            ui.weak(*l);
                        }
                    });
                }
            }
        }
    }

    fn inspector(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected.clone() else {
            ui.weak("click a clip in the timeline");
            return;
        };
        let Some(e) = self.engine.as_mut() else { return };
        if self.inspector_for != Some((id.clone(), self.revision)) {
            match ops::get_object(&e.project, &id) {
                Ok(v) => self.inspector_text = serde_json::to_string_pretty(&v).unwrap_or_default(),
                Err(_) => {
                    self.selected = None;
                    return;
                }
            }
            self.inspector_for = Some((id.clone(), self.revision));
        }
        ui.horizontal(|ui| {
            ui.strong(&id);
            if ui.button("Apply JSON").clicked() {
                match serde_json::from_str::<serde_json::Value>(&self.inspector_text) {
                    Ok(v) => match e.mutate("viewer edit", |p| ops::set_object(p, &id, v)) {
                        Ok(()) => self.status = format!("updated {id}"),
                        Err(err) => self.status = format!("invalid: {err}"),
                    },
                    Err(err) => self.status = format!("JSON error: {err}"),
                }
                self.revision += 1;
            }
            if ui.button("Delete").clicked() {
                let _ = e.mutate("viewer delete", |p| ops::remove(p, &id).map(|_| ()));
                self.selected = None;
                self.revision += 1;
            }
            if ui.button("Go to").clicked()
                && let Some(c) = e.project.clip(&id)
            {
                self.time = c.start;
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add(egui::TextEdit::multiline(&mut self.inspector_text).code_editor().desired_width(f32::INFINITY).desired_rows(30));
        });
    }

    fn library(&mut self, ui: &mut egui::Ui, effects: bool) {
        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("search…"));
        let Some(e) = self.engine.as_mut() else { return };
        let lib = e.library();
        let q = if self.search.trim().is_empty() { None } else { Some(self.search.trim().to_string()) };
        let target = self.selected.clone();
        let mut action: Option<(bool, String)> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            if effects {
                for fx in lib.search_effects(None, None, q.as_deref()) {
                    ui.horizontal(|ui| {
                        let can = target.is_some() && fx.kind == edits_fx::EffectKind::Filter;
                        if ui.add_enabled(can, egui::Button::new("+").small()).on_hover_text("add to selected clip").clicked() {
                            action = Some((true, fx.id.clone()));
                        }
                        ui.label(egui::RichText::new(&fx.id).strong()).on_hover_text(&fx.description);
                        ui.weak(format!("{} / {}", fx.kind.as_str(), fx.category));
                    });
                }
            } else {
                for p in lib.search_presets(None, q.as_deref()) {
                    ui.horizontal(|ui| {
                        let can = p.scope == edits_fx::PresetScope::Timeline || target.is_some();
                        if ui.add_enabled(can, egui::Button::new("apply").small()).on_hover_text(&p.description).clicked() {
                            action = Some((false, p.id.clone()));
                        }
                        ui.label(egui::RichText::new(&p.id).strong()).on_hover_text(&p.description);
                        ui.weak(&p.category);
                    });
                }
            }
        });
        if let Some((is_fx, id)) = action {
            let res = if is_fx {
                let t = target.clone().unwrap_or_default();
                e.mutate("viewer add effect", |p| ops::add_effect(p, &t, edits_core::EffectInstance::new("", &id), None).map(|_| ()))
                    .map_err(|e| e.to_string())
            } else {
                let scope = lib.preset(&id).map(|p| p.scope);
                let t = if scope == Some(edits_fx::PresetScope::Clip) { target.as_deref() } else { None };
                e.apply_preset(&id, t, &Default::default(), None).map(|_| ()).map_err(|e| e.to_string())
            };
            self.status = match res {
                Ok(()) => format!("applied {id}"),
                Err(err) => format!("{id}: {err}"),
            };
            self.revision += 1;
        }
    }
}

fn chrono_now() -> String {
    let s = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("{:02}:{:02}:{:02}", (s / 3600) % 24, (s / 60) % 60, s % 60)
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_file();
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) && !ctx.egui_wants_keyboard_input() {
            self.toggle_play();
        }
        self.advance();
        self.render_if_needed();

        egui::Panel::top("top").show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.weak(&self.status);
                if !self.warnings.is_empty() {
                    ui.colored_label(Color32::YELLOW, format!("⚠ {}", self.warnings.join(" · ")));
                }
            });
        });
        egui::Panel::bottom("timeline").resizable(true).default_size(220.0).show(ui, |ui| {
            self.transport(ui);
            egui::ScrollArea::vertical().show(ui, |ui| self.timeline(ui));
        });
        egui::Panel::right("side").resizable(true).default_size(380.0).show(ui, |ui| self.side_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(p) = &self.preview {
                let avail = ui.available_size();
                let (w, h) = (p.size.0 as f32, p.size.1 as f32);
                let s = (avail.x / w).min(avail.y / h).max(0.05);
                let size = Vec2::new(w * s, h * s);
                ui.centered_and_justified(|ui| {
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(p.id, size)).fit_to_exact_size(size));
                });
            } else {
                ui.centered_and_justified(|ui| ui.heading("No project — enter a path above and press Open"));
            }
        });
        if self.playing {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .init();
    let path = std::env::args().nth(1).map(PathBuf::from);
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_inner_size([1500.0, 920.0]).with_title("EditsEditor Viewer"),
        ..Default::default()
    };
    eframe::run_native("EditsEditor Viewer", options, Box::new(move |cc| Ok(Box::new(App::new(cc, path)))))
}
