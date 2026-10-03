//! Compact, token-efficient textual overview of a project for agents.

use std::fmt::Write;

use crate::model::{AssetKind, Clip, ClipSource, Composition, EffectInstance, Project};
use crate::property::Property;

fn ts(t: f64) -> String {
    format!("{t:.3}")
}

fn mmss(t: f64) -> String {
    let m = (t / 60.0).floor();
    format!("{}:{:05.2}", m as u64, t - m * 60.0)
}

fn fx_list(fx: &[EffectInstance]) -> String {
    fx.iter()
        .map(|e| {
            let mut s = format!("{}#{}", e.effect, e.id);
            if !e.enabled {
                s.push_str("(off)");
            }
            let animated = e.params.values().any(|p| p.is_animated());
            if animated {
                s.push('~');
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn source_desc(p: &Project, c: &Clip) -> String {
    match &c.source {
        ClipSource::Media { asset, no_audio, no_video } => {
            let name = p.assets.get(asset).map(|a| file_name(&a.path)).unwrap_or_else(|| "<missing>".into());
            let mut s = format!("media {asset}({name})");
            if *no_audio {
                s.push_str(" no_audio");
            }
            if *no_video {
                s.push_str(" no_video");
            }
            s
        }
        ClipSource::Solid { color, .. } => match color {
            Property::Static(c) => format!("solid {}", c.to_hex()),
            _ => "solid ~".into(),
        },
        ClipSource::Text(t) => {
            let mut txt: String = t.text.chars().take(40).collect();
            if t.text.chars().count() > 40 {
                txt.push('…');
            }
            format!("text \"{}\" {}px {}", txt.replace('\n', "⏎"), t.size, t.font)
        }
        ClipSource::Shape(s) => format!("shape {}", serde_json::to_value(&s.shape).ok().and_then(|v| v.get("type").and_then(|t| t.as_str()).map(String::from)).unwrap_or_default()),
        ClipSource::Generator { effect, .. } => format!("generator {effect}"),
        ClipSource::Comp { comp } => format!("comp {comp}"),
        ClipSource::Adjustment => "adjustment".into(),
    }
}

pub fn file_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or(path).to_string()
}

/// Render a compact multi-line summary.
pub fn summarize(p: &Project) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "Project \"{}\" (root: {})", p.meta.name, p.root);
    if !p.meta.notes.is_empty() {
        let _ = writeln!(o, "Notes: {}", p.meta.notes);
    }
    let t = &p.timing;
    if t.bpm.is_some() || !t.beats.is_empty() {
        let _ = writeln!(
            o,
            "Timing: {} BPM, {} beats, {} downbeats, offset {}{}{}",
            t.bpm.map(|b| format!("{b:.1}")).unwrap_or("?".into()),
            t.beats.len(),
            t.downbeats.len(),
            t.offset,
            t.source.as_ref().map(|s| format!(", source {s}")).unwrap_or_default(),
            if t.drops.is_empty() {
                String::new()
            } else {
                format!(", drops @ {}", t.drops.iter().map(|d| format!("{d:.2}")).collect::<Vec<_>>().join(" "))
            }
        );
        if !t.sections.is_empty() {
            let secs: Vec<String> =
                t.sections.iter().map(|s| format!("{}[{:.1}-{:.1}]", s.label, s.start, s.end)).collect();
            let _ = writeln!(o, "Sections: {}", secs.join(" "));
        }
    }
    if !p.assets.is_empty() {
        let _ = writeln!(o, "Assets ({}):", p.assets.len());
        for (id, a) in &p.assets {
            let mut d = format!("  {id} {:?} {}", a.kind, file_name(&a.path));
            if let Some(i) = &a.info {
                if i.width > 0 {
                    let _ = write!(d, " {}x{}", i.width, i.height);
                }
                if let Some(f) = i.fps {
                    if a.kind == AssetKind::Video {
                        let _ = write!(d, " {f:.3}fps");
                    }
                }
                if let Some(dur) = i.duration {
                    let _ = write!(d, " {}", mmss(dur));
                }
                if i.has_audio && a.kind == AssetKind::Video {
                    d.push_str(" +audio");
                }
                if !i.families.is_empty() {
                    let _ = write!(d, " fonts: {}", i.families.join(", "));
                }
            }
            if !a.tags.is_empty() {
                let _ = write!(d, " [{}]", a.tags.join(","));
            }
            let _ = writeln!(o, "{d}");
        }
    }
    for (cid, comp) in &p.compositions {
        comp_summary(&mut o, p, cid, comp);
    }
    if !p.custom_effects.is_empty() {
        let _ = writeln!(o, "Custom effects: {}", p.custom_effects.keys().cloned().collect::<Vec<_>>().join(", "));
    }
    if !p.custom_presets.is_empty() {
        let _ = writeln!(o, "Custom presets: {}", p.custom_presets.keys().cloned().collect::<Vec<_>>().join(", "));
    }
    if !p.variables.is_empty() {
        let _ = writeln!(o, "Variables: {}", serde_json::to_string(&p.variables).unwrap_or_default());
    }
    o
}

fn comp_summary(o: &mut String, p: &Project, cid: &str, comp: &Composition) {
    let _ = writeln!(
        o,
        "Comp {cid} \"{}\" {}x{} @{}fps {}s ({} frames) bg {}",
        comp.name,
        comp.width,
        comp.height,
        comp.fps,
        comp.duration,
        comp.frame_count(),
        comp.background.to_hex()
    );
    if !comp.effects.is_empty() {
        let _ = writeln!(o, "  comp fx: {}", fx_list(&comp.effects));
    }
    if !comp.markers.is_empty() {
        let m: Vec<String> = comp.markers.iter().map(|m| format!("{}@{:.2}{}", m.id, m.t, if m.label.is_empty() { String::new() } else { format!("\"{}\"", m.label) })).collect();
        let _ = writeln!(o, "  markers: {}", m.join(" "));
    }
    let _ = writeln!(o, "  tracks (bottom→top):");
    for t in &comp.tracks {
        let mut flags = vec![];
        if !t.enabled {
            flags.push("disabled");
        }
        if t.muted {
            flags.push("muted");
        }
        if t.solo {
            flags.push("solo");
        }
        if t.locked {
            flags.push("locked");
        }
        let _ = writeln!(
            o,
            "  - {} \"{}\" {} clips{}{}{}",
            t.id,
            t.name,
            t.clips.len(),
            if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(",")) },
            if t.blend_mode != Default::default() { format!(" blend={}", t.blend_mode.name()) } else { String::new() },
            if t.effects.is_empty() { String::new() } else { format!(" fx: {}", fx_list(&t.effects)) }
        );
        for c in &t.clips {
            let mut line = format!("      {} [{}–{}] {}", c.id, ts(c.start), ts(c.end()), source_desc(p, c));
            if !c.name.is_empty() {
                let _ = write!(line, " \"{}\"", c.name);
            }
            if c.source_in != 0.0 {
                let _ = write!(line, " in={:.3}", c.source_in);
            }
            match &c.speed {
                Property::Static(s) if *s != 1.0 => {
                    let _ = write!(line, " speed={s}");
                }
                Property::Animated(_) => line.push_str(" speed~"),
                _ => {}
            }
            if c.time_remap.is_some() {
                line.push_str(" remap~");
            }
            if c.reverse {
                line.push_str(" reverse");
            }
            if let Some(f) = c.freeze {
                let _ = write!(line, " freeze@{f}");
            }
            if !c.enabled {
                line.push_str(" (disabled)");
            }
            if c.hidden {
                line.push_str(" (hidden)");
            }
            if c.blend_mode != Default::default() {
                let _ = write!(line, " blend={}", c.blend_mode.name());
            }
            if c.opacity.is_animated() {
                line.push_str(" opacity~");
            } else if let Property::Static(v) = c.opacity {
                if v != 1.0 {
                    let _ = write!(line, " opacity={v}");
                }
            }
            let tr = &c.transform;
            let animated: Vec<&str> = [
                ("pos", tr.position.is_animated()),
                ("scale", tr.scale.is_animated()),
                ("rot", tr.rotation.is_animated()),
                ("rotX", tr.rotation_x.is_animated()),
                ("rotY", tr.rotation_y.is_animated()),
                ("z", tr.z.is_animated()),
            ]
            .iter()
            .filter(|x| x.1)
            .map(|x| x.0)
            .collect();
            if !animated.is_empty() {
                let _ = write!(line, " anim:{}", animated.join("/"));
            }
            if !c.effects.is_empty() {
                let _ = write!(line, " | fx: {}", fx_list(&c.effects));
            }
            if let Some(tr) = &c.transition_in {
                let _ = write!(line, " | in: {} {:.2}s", tr.effect, tr.duration);
            }
            if !c.masks.is_empty() {
                let _ = write!(line, " | masks:{}", c.masks.len());
            }
            if let Some(m) = &c.matte {
                let _ = write!(line, " | matte:{}", m.clip);
            }
            if c.motion_blur {
                line.push_str(" | mblur");
            }
            if c.echo.is_some() {
                line.push_str(" | echo");
            }
            if c.audio.mute {
                line.push_str(" | audio muted");
            }
            if !c.tags.is_empty() {
                let _ = write!(line, " [{}]", c.tags.join(","));
            }
            let _ = writeln!(o, "{line}");
        }
    }
}
