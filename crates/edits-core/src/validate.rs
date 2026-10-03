//! Structural validation: problems an agent should know about (not parse errors — those can't
//! happen once a project is loaded).

use serde::Serialize;

use crate::model::{ClipSource, Project};

#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize)]
pub struct Issue {
    pub severity: Severity,
    /// Id of the offending object (if any).
    pub object: Option<String>,
    pub message: String,
}

impl Issue {
    fn new(severity: Severity, object: Option<&str>, message: impl Into<String>) -> Self {
        Issue { severity, object: object.map(String::from), message: message.into() }
    }
}

pub fn validate(p: &Project) -> Vec<Issue> {
    let mut out = vec![];
    if !p.compositions.contains_key(&p.root) {
        out.push(Issue::new(Severity::Error, None, format!("root composition '{}' does not exist", p.root)));
    }
    let mut seen = std::collections::HashSet::new();
    for (cid, comp) in &p.compositions {
        if comp.width == 0 || comp.height == 0 || comp.width > 16384 || comp.height > 16384 {
            out.push(Issue::new(Severity::Error, Some(cid), "composition size must be 1..16384"));
        }
        if comp.fps <= 0.0 || comp.fps > 240.0 {
            out.push(Issue::new(Severity::Error, Some(cid), "fps must be in (0, 240]"));
        }
        if comp.duration <= 0.0 {
            out.push(Issue::new(Severity::Error, Some(cid), "duration must be > 0"));
        }
        for t in &comp.tracks {
            if !seen.insert(t.id.clone()) {
                out.push(Issue::new(Severity::Error, Some(&t.id), "duplicate id"));
            }
            let mut prev_end: Option<(f64, &str)> = None;
            for c in &t.clips {
                if !seen.insert(c.id.clone()) {
                    out.push(Issue::new(Severity::Error, Some(&c.id), "duplicate id"));
                }
                for e in &c.effects {
                    if !e.id.is_empty() && !seen.insert(e.id.clone()) {
                        out.push(Issue::new(Severity::Error, Some(&e.id), "duplicate id"));
                    }
                }
                if c.duration <= 0.0 {
                    out.push(Issue::new(Severity::Error, Some(&c.id), "clip duration must be > 0"));
                }
                if c.start >= comp.duration {
                    out.push(Issue::new(Severity::Warning, Some(&c.id), format!("clip starts after the composition ends ({}s)", comp.duration)));
                } else if c.end() > comp.duration + 1e-6 {
                    out.push(Issue::new(Severity::Info, Some(&c.id), "clip extends past the composition end"));
                }
                if let Some((pe, pid)) = prev_end {
                    if c.start < pe - 1e-6 && c.enabled && c.transition_in.is_none() {
                        out.push(Issue::new(
                            Severity::Info,
                            Some(&c.id),
                            format!("overlaps {pid} on the same track (later clip draws on top)"),
                        ));
                    }
                }
                if c.enabled {
                    prev_end = Some((c.end(), &c.id));
                }
                match &c.source {
                    ClipSource::Media { asset, .. } => {
                        if let Some(a) = p.assets.get(asset) {
                            if let (Some(info), None, None) = (&a.info, &c.time_remap, c.freeze) {
                                if let (Some(d), crate::property::Property::Static(s)) = (info.duration, &c.speed) {
                                    let need = c.source_in + s.abs() * c.duration;
                                    if need > d + 0.05 && c.end_behavior == crate::model::EndBehavior::Hold {
                                        out.push(Issue::new(
                                            Severity::Info,
                                            Some(&c.id),
                                            format!("source runs out at {:.2}s local; last frame is held", ((d - c.source_in) / s.abs()).max(0.0)),
                                        ));
                                    }
                                }
                            }
                        } else {
                            out.push(Issue::new(Severity::Error, Some(&c.id), format!("unknown asset '{asset}'")));
                        }
                    }
                    ClipSource::Comp { comp: inner } => {
                        if !p.compositions.contains_key(inner) {
                            out.push(Issue::new(Severity::Error, Some(&c.id), format!("unknown composition '{inner}'")));
                        } else if inner == cid {
                            out.push(Issue::new(Severity::Error, Some(&c.id), "composition cannot contain itself"));
                        }
                    }
                    _ => {}
                }
                if let Some(m) = &c.matte {
                    if p.clip(&m.clip).is_none() {
                        out.push(Issue::new(Severity::Error, Some(&c.id), format!("matte clip '{}' not found", m.clip)));
                    }
                }
                if let Some(tr) = &c.transition_in {
                    if tr.duration > c.duration {
                        out.push(Issue::new(Severity::Warning, Some(&c.id), "transition is longer than the clip"));
                    }
                }
            }
        }
    }
    // nested comp cycles
    for cid in p.compositions.keys() {
        let mut stack = vec![(cid.clone(), 0)];
        while let Some((c, depth)) = stack.pop() {
            if depth > 16 {
                out.push(Issue::new(Severity::Error, Some(cid), "nested composition cycle or depth > 16"));
                break;
            }
            if let Some(comp) = p.compositions.get(&c) {
                for t in &comp.tracks {
                    for cl in &t.clips {
                        if let ClipSource::Comp { comp: inner } = &cl.source {
                            stack.push((inner.clone(), depth + 1));
                        }
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.severity.cmp(&a.severity));
    out
}
