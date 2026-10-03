//! Text layout and rasterization (CPU): cosmic-text shaping (complex scripts, CJK/emoji
//! fallback), per-unit animators, letter spacing, exact-distance strokes, soft shadows,
//! gradients and background boxes. Layouts are cached; only animated text re-rasterizes.

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, LayoutGlyph, Metrics, Shaping, Style, SwashCache, SwashContent, Weight};
use edits_core::{Color, TextAlign, TextOrder, TextSource, TextUnit, color::srgb_to_linear};
use edits_media::Frame;
use lru::LruCache;
use parking_lot::Mutex;

struct Line {
    y: f32,
    w: f32,
    glyphs: Vec<usize>,
}

struct Layout {
    glyphs: Vec<LayoutGlyph>,
    /// Per glyph: (char unit index, word index, line index, is_whitespace)
    units: Vec<(usize, usize, usize, bool)>,
    lines: Vec<Line>,
    #[allow(dead_code)]
    width: f32,
    height: f32,
    n_chars: usize,
    n_words: usize,
}

pub struct TextRenderer {
    fonts: Mutex<Option<FontSystem>>,
    swash: Mutex<SwashCache>,
    layouts: Mutex<LruCache<u64, Arc<Layout>>>,
    loaded: Mutex<HashSet<PathBuf>>,
}

/// Dynamic (animatable) values resolved by the engine for this frame.
pub struct TextFrameParams {
    pub color: Color,
    pub letter_spacing: f64,
    /// Clip-local time and clip duration (for animators).
    pub time: f64,
    pub duration: f64,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_of(h: impl Hash) -> u64 {
    let mut s = std::collections::hash_map::DefaultHasher::new();
    h.hash(&mut s);
    s.finish()
}

fn hash01(a: u64, b: u64) -> f64 {
    let mut x = a.wrapping_mul(0x9E3779B97F4A7C15) ^ b.wrapping_mul(0xC2B2AE3D27D4EB4F);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

impl TextRenderer {
    pub fn new() -> Self {
        TextRenderer {
            fonts: Mutex::new(None),
            swash: Mutex::new(SwashCache::new()),
            layouts: Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())),
            loaded: Mutex::new(HashSet::new()),
        }
    }

    fn with_fonts<R>(&self, f: impl FnOnce(&mut FontSystem) -> R) -> R {
        let mut g = self.fonts.lock();
        let fs = g.get_or_insert_with(FontSystem::new);
        f(fs)
    }

    /// Load a font file (ttf/otf/ttc); returns the family names it provides.
    pub fn load_font_file(&self, path: &Path) -> Vec<String> {
        let first = self.loaded.lock().insert(path.to_path_buf());
        self.with_fonts(|fs| {
            let db = fs.db_mut();
            let before: HashSet<_> = db.faces().map(|f| f.id).collect();
            if first {
                let _ = db.load_font_file(path);
            }
            let mut fams: Vec<String> = db
                .faces()
                .filter(|f| !before.contains(&f.id) || matches!(&f.source, cosmic_text::fontdb::Source::File(p) if p == path))
                .flat_map(|f| f.families.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>())
                .collect();
            fams.sort();
            fams.dedup();
            fams
        })
    }

    /// All available font family names.
    pub fn families(&self) -> Vec<String> {
        self.with_fonts(|fs| {
            let mut v: Vec<String> = fs.db().faces().flat_map(|f| f.families.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()).collect();
            v.sort();
            v.dedup();
            v
        })
    }

    fn layout(&self, src: &TextSource) -> Arc<Layout> {
        let key = hash_of((
            &src.text,
            &src.font,
            src.weight,
            src.italic,
            src.size.to_bits(),
            src.line_height.to_bits(),
            src.max_width.map(f64::to_bits),
            src.uppercase,
        ));
        if let Some(l) = self.layouts.lock().get(&key) {
            return l.clone();
        }
        let text = if src.uppercase { src.text.to_uppercase() } else { src.text.clone() };
        let size = src.size.max(1.0) as f32;
        let line_h = size * src.line_height.max(0.5) as f32;
        let layout = self.with_fonts(|fs| {
            let mut buf = Buffer::new(fs, Metrics::new(size, line_h));
            buf.set_size(src.max_width.map(|w| w as f32), None);
            let fam = match src.font.to_ascii_lowercase().as_str() {
                "sans-serif" | "sans" | "" => Family::SansSerif,
                "serif" => Family::Serif,
                "monospace" | "mono" => Family::Monospace,
                "cursive" => Family::Cursive,
                "fantasy" => Family::Fantasy,
                _ => Family::Name(&src.font),
            };
            // Snap to the nearest weight the family really has; otherwise cosmic-text falls back
            // glyph-by-glyph to other fonts with different metrics (wide gaps, mixed styles).
            let mut fam_name = match fam {
                Family::Name(n) => n.to_string(),
                other => fs.db().family_name(&other).to_string(),
            };
            let has =
                |db: &cosmic_text::fontdb::Database, n: &str| db.faces().any(|f| f.families.iter().any(|(x, _)| x.eq_ignore_ascii_case(n)));
            if !has(fs.db(), &fam_name) {
                // resolve through the matcher (handles generic names and missing families)
                let q = cosmic_text::fontdb::Query { families: &[fam, Family::SansSerif], ..Default::default() };
                let resolved = fs.db().query(&q).and_then(|id| fs.db().face(id)).and_then(|f| f.families.first().map(|(n, _)| n.clone()));
                let fallback = fs
                    .db()
                    .faces()
                    .find(|f| {
                        f.families.iter().any(|(n, _)| {
                            let l = n.to_ascii_lowercase();
                            l.contains("segoe ui")
                                || l.contains("noto sans")
                                || l.contains("dejavu sans")
                                || l.contains("liberation sans")
                                || l == "arial"
                        })
                    })
                    .and_then(|f| f.families.first().map(|(n, _)| n.clone()));
                if let Some(r) = resolved.or(fallback) {
                    fam_name = r;
                }
            }
            let want = src.weight as i32;
            let weight = fs
                .db()
                .faces()
                .filter(|f| f.families.iter().any(|(n, _)| n.eq_ignore_ascii_case(&fam_name)))
                .map(|f| f.weight.0)
                .min_by_key(|w| (*w as i32 - want).abs() * 2 + if (*w as i32) < want { 1 } else { 0 })
                .unwrap_or(src.weight);
            let attrs = Attrs::new().family(Family::Name(&fam_name)).weight(Weight(weight)).style(if src.italic {
                Style::Italic
            } else {
                Style::Normal
            });
            buf.set_text(&text, &attrs, Shaping::Advanced, None);
            buf.shape_until_scroll(fs, false);
            // char/word indices by byte offset
            let mut char_idx = HashMap::new();
            let mut word_idx = HashMap::new();
            let (mut ci, mut wi, mut in_word) = (0usize, 0usize, false);
            for (b, ch) in text.char_indices() {
                if ch.is_whitespace() {
                    if in_word {
                        wi += 1;
                    }
                    in_word = false;
                } else {
                    in_word = true;
                }
                char_idx.insert(b, ci);
                word_idx.insert(b, wi);
                if !ch.is_whitespace() {
                    ci += 1;
                }
            }
            let mut glyphs = vec![];
            let mut units = vec![];
            let mut lines: Vec<Line> = vec![];
            let mut width = 0f32;
            let mut height = 0f32;
            // byte offset of each paragraph (buffer line) in `text`
            let mut para_offsets = vec![0usize];
            for (b, ch) in text.char_indices() {
                if ch == '\n' {
                    para_offsets.push(b + 1);
                }
            }
            for (li, run) in buf.layout_runs().enumerate() {
                let mut line = Line { y: run.line_y, w: run.line_w, glyphs: vec![] };
                // glyph.start is relative to the paragraph text
                let line_offset = para_offsets.get(run.line_i).copied().unwrap_or(0);
                for g in run.glyphs.iter() {
                    let b = (line_offset + g.start).min(text.len());
                    let ws = text.get(b..).and_then(|t| t.chars().next()).is_some_and(char::is_whitespace);
                    line.glyphs.push(glyphs.len());
                    units.push((*char_idx.get(&b).unwrap_or(&0), *word_idx.get(&b).unwrap_or(&0), li, ws));
                    glyphs.push(g.clone());
                }
                width = width.max(run.line_w);
                height = height.max(run.line_top + run.line_height);
                lines.push(line);
            }
            Layout { glyphs, units, lines, width, height, n_chars: ci, n_words: wi + 1 }
        });
        let l = Arc::new(layout);
        self.layouts.lock().put(key, l.clone());
        l
    }

    /// Whether the text needs re-rasterizing every frame.
    pub fn is_animated(src: &TextSource) -> bool {
        src.animator.is_some() || src.color.is_animated() || src.letter_spacing.is_animated()
    }

    /// Rasterize the text. The returned frame is premultiplied and centered on the text block.
    pub fn render(&self, src: &TextSource, p: &TextFrameParams) -> Frame {
        let lay = self.layout(src);
        let spacing = p.letter_spacing as f32;
        let size = src.size.max(1.0) as f32;
        // block size including letter spacing
        let line_extra = |l: &Line| spacing * (l.glyphs.len().saturating_sub(1)) as f32;
        let block_w = lay.lines.iter().map(|l| l.w + line_extra(l)).fold(0.0, f32::max).max(1.0);
        let block_h = lay.height.max(size);
        let stroke_w = src.stroke.as_ref().map(|s| s.width as f32).unwrap_or(0.0);
        let (sh_off, sh_blur) =
            src.shadow.as_ref().map(|s| ((s.offset.x().abs().max(s.offset.y().abs())) as f32, s.blur as f32)).unwrap_or((0.0, 0.0));
        let anim_margin = src.animator.as_ref().map(|a| {
            let from = a.from.offset.x().abs().max(a.from.offset.y().abs()) as f32;
            let out = a.out.as_ref().map(|o| o.to.offset.x().abs().max(o.to.offset.y().abs()) as f32).unwrap_or(0.0);
            let scale = (a.from.scale.max(a.out.as_ref().map(|o| o.to.scale).unwrap_or(1.0)) as f32 - 1.0).max(0.0) * size;
            from.max(out) + scale + a.wave.map(|w| w[0].abs() as f32).unwrap_or(0.0) + a.jitter as f32 + size * 0.3
        });
        let bg_pad = src.background.as_ref().map(|b| (b.padding.x() as f32, b.padding.y() as f32)).unwrap_or((0.0, 0.0));
        let margin = (stroke_w + sh_off + sh_blur * 2.0 + anim_margin.unwrap_or(0.0) + size * 0.25 + 4.0).ceil();
        let w = (block_w + 2.0 * (margin + bg_pad.0)).ceil() as usize;
        let h = (block_h + 2.0 * (margin + bg_pad.1)).ceil() as usize;
        let (w, h) = (w.clamp(1, 8192), h.clamp(1, 8192));
        let ox = (w as f32 - block_w) / 2.0;
        let oy = (h as f32 - block_h) / 2.0;

        let mut alpha = vec![0f32; w * h];
        let mut color_layer: Vec<[f32; 4]> = vec![];
        let mut fonts_guard = self.fonts.lock();
        let fs = fonts_guard.get_or_insert_with(FontSystem::new);
        let mut swash = self.swash.lock();
        let n_units = |u: TextUnit| match u {
            TextUnit::Char => lay.n_chars.max(1),
            TextUnit::Word => lay.n_words.max(1),
            TextUnit::Line => lay.lines.len().max(1),
        };
        for line in &lay.lines {
            let lx = match src.align {
                TextAlign::Left => 0.0,
                TextAlign::Center => (block_w - (line.w + line_extra(line))) / 2.0,
                TextAlign::Right => block_w - (line.w + line_extra(line)),
            };
            for (k, &gi) in line.glyphs.iter().enumerate() {
                let g = &lay.glyphs[gi];
                let (ci, wi, li, ws) = lay.units[gi];
                if ws {
                    continue;
                }
                let gx = ox + lx + spacing * k as f32;
                let gy = oy + line.y;
                // animator state
                let (mut op, mut dx, mut dy, mut sc, mut rot) = (1.0f64, 0.0f64, 0.0f64, 1.0f64, 0.0f64);
                if let Some(a) = &src.animator {
                    let unit = match a.unit {
                        TextUnit::Char => ci,
                        TextUnit::Word => wi,
                        TextUnit::Line => li,
                    };
                    let nu = n_units(a.unit);
                    let order = |i: usize| -> f64 {
                        let c = (nu as f64 - 1.0) / 2.0;
                        match a.order {
                            TextOrder::Forward => i as f64,
                            TextOrder::Backward => (nu - 1 - i.min(nu - 1)) as f64,
                            TextOrder::CenterOut => (i as f64 - c).abs(),
                            TextOrder::EdgesIn => c - (i as f64 - c).abs(),
                            TextOrder::Random => (hash01(i as u64, 7) * nu as f64).floor(),
                        }
                    };
                    let oi = order(unit);
                    let delay = a.start + oi * a.stagger;
                    let pr = if a.duration <= 0.0 {
                        if p.time >= delay { 1.0 } else { 0.0 }
                    } else {
                        ((p.time - delay) / a.duration).clamp(0.0, 1.0)
                    };
                    let e = a.ease.apply(pr, a.duration);
                    op = a.from.opacity + (1.0 - a.from.opacity) * e;
                    dx = a.from.offset.x() * (1.0 - e);
                    dy = a.from.offset.y() * (1.0 - e);
                    sc = a.from.scale + (1.0 - a.from.scale) * e;
                    rot = a.from.rotation * (1.0 - e);
                    if let Some(o) = &a.out {
                        let total = (nu as f64 - 1.0) * o.stagger + o.duration;
                        let od = (p.duration - total).max(0.0) + oi * o.stagger;
                        let q = if o.duration <= 0.0 {
                            if p.time >= od { 1.0 } else { 0.0 }
                        } else {
                            ((p.time - od) / o.duration).clamp(0.0, 1.0)
                        };
                        let e2 = o.ease.apply(q, o.duration);
                        op += (o.to.opacity - op) * e2;
                        dx += o.to.offset.x() * e2;
                        dy += o.to.offset.y() * e2;
                        sc *= 1.0 + (o.to.scale - 1.0) * e2;
                        rot += o.to.rotation * e2;
                    }
                    if let Some([amp, freq, phase]) = a.wave {
                        dy += amp * (std::f64::consts::TAU * freq * p.time + phase * unit as f64).sin();
                    }
                    if a.jitter > 0.0 {
                        let step = (p.time * a.jitter_rate).floor() as u64;
                        dx += (hash01(unit as u64, step) - 0.5) * 2.0 * a.jitter;
                        dy += (hash01(unit as u64 + 1000, step) - 0.5) * 2.0 * a.jitter;
                    }
                }
                if op <= 0.001 {
                    continue;
                }
                let phys = g.physical((gx, gy), 1.0);
                let Some(img) = swash.get_image(fs, phys.cache_key).as_ref() else { continue };
                let pl = img.placement;
                if pl.width == 0 || pl.height == 0 {
                    continue;
                }
                let x0 = phys.x as f32 + pl.left as f32;
                let y0 = phys.y as f32 - pl.top as f32;
                // pivot: glyph center
                let cx = gx + g.x + g.w / 2.0;
                let cy = gy - size * 0.35;
                let transformed = (sc - 1.0).abs() > 1e-4 || rot.abs() > 1e-4 || dx.abs() > 1e-4 || dy.abs() > 1e-4;
                let (cs, sn) = ((-rot.to_radians()).cos() as f32, (-rot.to_radians()).sin() as f32);
                let inv_s = 1.0 / sc.max(0.01) as f32;
                // destination bbox
                let corners =
                    [(x0, y0), (x0 + pl.width as f32, y0), (x0, y0 + pl.height as f32), (x0 + pl.width as f32, y0 + pl.height as f32)];
                let fwd = |x: f32, y: f32| -> (f32, f32) {
                    let (rx, ry) = ((x - cx) * sc as f32, (y - cy) * sc as f32);
                    let (c2, s2) = (rot.to_radians().cos() as f32, rot.to_radians().sin() as f32);
                    (rx * c2 - ry * s2 + cx + dx as f32, rx * s2 + ry * c2 + cy + dy as f32)
                };
                let (mut bx0, mut by0, mut bx1, mut by1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
                for (x, y) in corners {
                    let (a, b) = if transformed { fwd(x, y) } else { (x, y) };
                    bx0 = bx0.min(a);
                    by0 = by0.min(b);
                    bx1 = bx1.max(a);
                    by1 = by1.max(b);
                }
                let (ix0, iy0) = ((bx0.floor() as i32 - 1).max(0), (by0.floor() as i32 - 1).max(0));
                let (ix1, iy1) = ((bx1.ceil() as i32 + 1).min(w as i32 - 1), (by1.ceil() as i32 + 1).min(h as i32 - 1));
                let is_color = matches!(img.content, SwashContent::Color);
                if is_color && color_layer.is_empty() {
                    color_layer = vec![[0.0; 4]; w * h];
                }
                let sample = |sx: f32, sy: f32| -> [f32; 4] {
                    // bilinear sample of the glyph image at glyph-local coords
                    let fx = sx - 0.5;
                    let fy = sy - 0.5;
                    let x = fx.floor() as i32;
                    let y = fy.floor() as i32;
                    let tx = fx - x as f32;
                    let ty = fy - y as f32;
                    let get = |xx: i32, yy: i32| -> [f32; 4] {
                        if xx < 0 || yy < 0 || xx >= pl.width as i32 || yy >= pl.height as i32 {
                            return [0.0; 4];
                        }
                        let i = (yy as usize) * pl.width as usize + xx as usize;
                        match img.content {
                            SwashContent::Color => {
                                let p = &img.data[i * 4..i * 4 + 4];
                                let a = p[3] as f32 / 255.0;
                                [p[0] as f32 / 255.0 * a, p[1] as f32 / 255.0 * a, p[2] as f32 / 255.0 * a, a]
                            }
                            SwashContent::SubpixelMask => {
                                let p = &img.data[i * 4..i * 4 + 3];
                                let a = (p[0] as f32 + p[1] as f32 + p[2] as f32) / (3.0 * 255.0);
                                [a, a, a, a]
                            }
                            SwashContent::Mask => {
                                let a = img.data[i] as f32 / 255.0;
                                [a, a, a, a]
                            }
                        }
                    };
                    let (a, b, c, d) = (get(x, y), get(x + 1, y), get(x, y + 1), get(x + 1, y + 1));
                    let mut o = [0.0; 4];
                    for k in 0..4 {
                        o[k] = (a[k] * (1.0 - tx) + b[k] * tx) * (1.0 - ty) + (c[k] * (1.0 - tx) + d[k] * tx) * ty;
                    }
                    o
                };
                for py in iy0..=iy1 {
                    for px in ix0..=ix1 {
                        let (x, y) = (px as f32 + 0.5, py as f32 + 0.5);
                        let (lx_, ly_) = if transformed {
                            let (ux, uy) = (x - cx - dx as f32, y - cy - dy as f32);
                            ((ux * cs - uy * sn) * inv_s + cx, (ux * sn + uy * cs) * inv_s + cy)
                        } else {
                            (x, y)
                        };
                        let s = sample(lx_ - x0, ly_ - y0);
                        if s[3] <= 0.0 {
                            continue;
                        }
                        let idx = py as usize * w + px as usize;
                        let a = s[3] * op as f32;
                        if is_color {
                            let d = &mut color_layer[idx];
                            for k in 0..4 {
                                d[k] = s[k] * op as f32 + d[k] * (1.0 - a);
                            }
                        } else {
                            alpha[idx] = a + alpha[idx] * (1.0 - a);
                        }
                    }
                }
            }
        }
        drop(swash);
        drop(fonts_guard);

        // ---- compose layers ----
        let mut out = vec![[0f32; 4]; w * h];
        let lin = |c: &Color| -> [f32; 4] {
            let a = c.0[3];
            [srgb_to_linear(c.0[0]) * a, srgb_to_linear(c.0[1]) * a, srgb_to_linear(c.0[2]) * a, a]
        };
        // background box
        if let Some(bg) = &src.background {
            let c = lin(&bg.color);
            let (hx, hy) = (block_w / 2.0 + bg_pad.0, block_h / 2.0 + bg_pad.1);
            let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
            let r = (bg.radius as f32).min(hx).min(hy);
            for y in 0..h {
                for x in 0..w {
                    let qx = ((x as f32 + 0.5 - cx).abs() - hx + r).max(0.0);
                    let qy = ((y as f32 + 0.5 - cy).abs() - hy + r).max(0.0);
                    let inner = ((x as f32 + 0.5 - cx).abs() - hx + r).max((y as f32 + 0.5 - cy).abs() - hy + r).min(0.0);
                    let d = (qx * qx + qy * qy).sqrt() + inner - r;
                    let k = (0.5 - d).clamp(0.0, 1.0);
                    let o = &mut out[y * w + x];
                    for i in 0..4 {
                        o[i] = c[i] * k + o[i] * (1.0 - c[3] * k);
                    }
                }
            }
        }
        // stroke via exact Euclidean distance transform of the glyph coverage
        let stroke_alpha: Option<Vec<f32>> = src.stroke.as_ref().filter(|s| s.width > 0.0).map(|s| {
            let dist = edt(&alpha, w, h);
            let sw = s.width as f32;
            dist.iter().zip(alpha.iter()).map(|(d, a)| ((sw + 0.5 - d).clamp(0.0, 1.0)).max(*a)).collect()
        });
        // shadow
        if let Some(sh) = &src.shadow {
            let base: Vec<f32> = match &stroke_alpha {
                Some(s) => s.clone(),
                None => alpha.clone(),
            };
            let blurred = box_blur(&base, w, h, (sh.blur as f32 / 2.0).max(0.0) as usize);
            let c = lin(&sh.color);
            let (sx, sy) = (sh.offset.x().round() as i32, sh.offset.y().round() as i32);
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let (xx, yy) = (x - sx, y - sy);
                    if xx < 0 || yy < 0 || xx >= w as i32 || yy >= h as i32 {
                        continue;
                    }
                    let a = blurred[yy as usize * w + xx as usize];
                    if a <= 0.0 {
                        continue;
                    }
                    let o = &mut out[y as usize * w + x as usize];
                    for i in 0..4 {
                        o[i] = c[i] * a + o[i] * (1.0 - c[3] * a);
                    }
                }
            }
        }
        if let (Some(sa), Some(s)) = (&stroke_alpha, &src.stroke) {
            let c = lin(&s.color);
            for (i, a) in sa.iter().enumerate() {
                if *a > 0.0 {
                    let o = &mut out[i];
                    for k in 0..4 {
                        o[k] = c[k] * a + o[k] * (1.0 - c[3] * a);
                    }
                }
            }
        }
        // fill (solid or vertical gradient)
        let top_y = oy;
        for y in 0..h {
            let fill = match &src.gradient {
                Some([a, b]) => {
                    let t = ((y as f32 - top_y) / block_h.max(1.0)).clamp(0.0, 1.0) as f64;
                    lin(&a.lerp(b, t))
                }
                None => lin(&p.color),
            };
            for x in 0..w {
                let a = alpha[y * w + x];
                if a > 0.0 {
                    let o = &mut out[y * w + x];
                    for k in 0..4 {
                        o[k] = fill[k] * a + o[k] * (1.0 - fill[3] * a);
                    }
                }
            }
        }
        if !color_layer.is_empty() {
            for (o, c) in out.iter_mut().zip(color_layer.iter()) {
                for k in 0..4 {
                    o[k] = c[k] + o[k] * (1.0 - c[3]);
                }
            }
        }
        // linear premultiplied -> sRGB premultiplied-equivalent: store straight sRGB then mark straight
        let mut data = Vec::with_capacity(w * h * 4);
        for o in out {
            let a = o[3].clamp(0.0, 1.0);
            if a <= 0.0 {
                data.extend_from_slice(&[0, 0, 0, 0]);
                continue;
            }
            let enc = |v: f32| (edits_core::color::linear_to_srgb((v / a).clamp(0.0, 1.0)) * 255.0 + 0.5) as u8;
            data.extend_from_slice(&[enc(o[0]), enc(o[1]), enc(o[2]), (a * 255.0 + 0.5) as u8]);
        }
        Frame::new(w as u32, h as u32, data, false)
    }
}

/// Exact Euclidean distance (pixels) from each pixel to the nearest pixel with coverage >= 0.5
/// (Felzenszwalb & Huttenlocher), with sub-pixel correction from partial coverage.
pub fn edt(alpha: &[f32], w: usize, h: usize) -> Vec<f32> {
    const INF: f32 = 1e20;
    let mut f: Vec<f32> = alpha.iter().map(|a| if *a >= 0.5 { 0.0 } else { INF }).collect();
    let n = w.max(h);
    let mut d = vec![0f32; n];
    let mut v = vec![0usize; n];
    let mut z = vec![0f32; n + 1];
    let mut line = vec![0f32; n];
    let mut dt1 = |f: &mut [f32], len: usize, stride: usize, off: usize| {
        for i in 0..len {
            line[i] = f[off + i * stride];
        }
        let mut k = 0usize;
        v[0] = 0;
        z[0] = f32::NEG_INFINITY;
        z[1] = f32::INFINITY;
        for q in 1..len {
            let qf = q as f32;
            loop {
                let p = v[k];
                let pf = p as f32;
                let s = ((line[q] + qf * qf) - (line[p] + pf * pf)) / (2.0 * qf - 2.0 * pf);
                if s <= z[k] {
                    if k == 0 {
                        break;
                    }
                    k -= 1;
                } else {
                    k += 1;
                    v[k] = q;
                    z[k] = s;
                    z[k + 1] = f32::INFINITY;
                    break;
                }
            }
        }
        k = 0;
        for q in 0..len {
            while z[k + 1] < q as f32 {
                k += 1;
            }
            let p = v[k];
            d[q] = (q as f32 - p as f32).powi(2) + line[p];
        }
        for i in 0..len {
            f[off + i * stride] = d[i];
        }
    };
    for x in 0..w {
        dt1(&mut f, h, w, x);
    }
    for y in 0..h {
        dt1(&mut f, w, 1, y * w);
    }
    f.iter().zip(alpha.iter()).map(|(d2, a)| (d2.sqrt() - (a - 0.5).max(-0.5)).max(0.0)).collect()
}

/// Three-pass box blur approximating a gaussian.
pub fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    if r == 0 {
        return src.to_vec();
    }
    let mut a = src.to_vec();
    let mut b = vec![0f32; a.len()];
    for _ in 0..3 {
        // horizontal
        for y in 0..h {
            let row = &a[y * w..(y + 1) * w];
            let mut acc = 0f32;
            for x in 0..=r.min(w - 1) {
                acc += row[x];
            }
            for x in 0..w {
                let lo = x as i64 - r as i64 - 1;
                let hi = x + r;
                if x > 0 {
                    if hi < w {
                        acc += row[hi];
                    }
                    if lo >= 0 {
                        acc -= row[lo as usize];
                    }
                }
                b[y * w + x] = acc / (2 * r + 1) as f32;
            }
        }
        // vertical
        for x in 0..w {
            let mut acc = 0f32;
            for y in 0..=r.min(h - 1) {
                acc += b[y * w + x];
            }
            for y in 0..h {
                let lo = y as i64 - r as i64 - 1;
                let hi = y + r;
                if y > 0 {
                    if hi < h {
                        acc += b[hi * w + x];
                    }
                    if lo >= 0 {
                        acc -= b[lo as usize * w + x];
                    }
                }
                a[y * w + x] = acc / (2 * r + 1) as f32;
            }
        }
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edt_basic() {
        let w = 9;
        let mut a = vec![0f32; w * w];
        a[4 * w + 4] = 1.0;
        let d = edt(&a, w, w);
        assert!(d[4 * w + 4] < 0.01);
        assert!((d[4 * w + 7] - 3.0).abs() < 0.6, "{}", d[4 * w + 7]);
    }

    #[test]
    fn renders_text() {
        let r = TextRenderer::new();
        if r.families().is_empty() {
            eprintln!("no fonts on this system; skipping");
            return;
        }
        let mut src = TextSource::simple("Hello 世界", 48.0);
        src.stroke = Some(edits_core::TextStroke { color: Color::BLACK, width: 3.0 });
        let f = r.render(&src, &TextFrameParams { color: Color::WHITE, letter_spacing: 2.0, time: 0.0, duration: 1.0 });
        assert!(f.width > 100 && f.height > 40);
        let covered = f.data.chunks_exact(4).filter(|p| p[3] > 128).count();
        assert!(covered > 200, "{covered}");
    }
}
