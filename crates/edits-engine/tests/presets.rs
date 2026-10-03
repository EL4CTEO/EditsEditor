use std::path::Path;

use edits_core::{Project, ops, validate::Severity};
use edits_engine::{AnalysisOptions, Engine, ImportOptions, OutputMode};
use edits_fx::PresetScope;
use serde_json::json;

fn make_media(dir: &Path) -> bool {
    let Ok(ff) = edits_media::Ffmpeg::locate() else { return false };
    let ok = |args: &[&str], out: &Path| ff.cmd().arg("-y").args(args).arg(out).status().map(|s| s.success()).unwrap_or(false);
    // footage with 3 hard cuts (scene detection) and a moving pattern
    ok(&["-f", "lavfi", "-i", "testsrc2=s=320x180:r=24:d=2", "-f", "lavfi", "-i", "mandelbrot=s=320x180:r=24", "-f", "lavfi", "-i", "smptebars=s=320x180:r=24:d=2",
        "-filter_complex", "[1:v]trim=duration=2,setpts=PTS-STARTPTS[m];[0:v][m][2:v]concat=n=3:v=1:a=0[v]", "-map", "[v]", "-c:v", "libx264", "-pix_fmt", "yuv420p"], &dir.join("footage.mp4"))
        && ok(&["-f", "lavfi", "-i", "aevalsrc='0.9*sin(2*PI*55*t)*exp(-25*mod(t,0.5))*(1+2*gte(t,6))':s=48000:d=12"], &dir.join("song.wav"))
        && std::fs::write(dir.join("words.srt"), "1\n00:00:01,000 --> 00:00:02,500\nhello there\n\n2\n00:00:03,000 --> 00:00:04,000\nsecond line\n").is_ok()
}

fn base(dir: &Path) -> Option<Engine> {
    if !make_media(dir) {
        return None;
    }
    let mut e = Engine::create(&dir.join("p.edits.json"), Project::new("presets", 320, 180, 24.0, 6.0)).unwrap();
    e.autosave = false;
    if e.gpu_info().is_err() {
        return None;
    }
    e.import(&[dir.to_string_lossy().to_string()], &ImportOptions::default()).unwrap();
    e.mutate("music", |p| ops::add_clip(p, None, ops::TrackTarget::New, json!({"start": 0, "duration": 6, "source": {"type": "media", "asset": "song"}}))).unwrap();
    e.analyze_audio("song", &AnalysisOptions::default(), true, None).unwrap();
    // make sure there's at least one drop for drop-based presets
    e.mutate("drop", |p| {
        if p.timing.drops.is_empty() {
            p.timing.drops = vec![3.0];
        }
        p.timing.sections = vec![
            edits_core::Section { start: 0.0, end: 3.0, label: "build".into(), energy: 0.4 },
            edits_core::Section { start: 3.0, end: 12.0, label: "drop".into(), energy: 1.0 },
        ];
        Ok(())
    })
    .unwrap();
    Some(e)
}

#[test]
fn every_preset_applies_and_renders() {
    let dir = tempfile::tempdir().unwrap();
    let Some(mut e) = base(dir.path()) else {
        eprintln!("ffmpeg/gpu missing; skipping");
        return;
    };
    let lib = e.library();
    let snapshot = e.project.clone();
    let mut failures = vec![];
    let mut count = 0;
    for def in lib.presets() {
        e.project = snapshot.clone();
        let mut args = serde_json::Map::new();
        let target = match def.scope {
            PresetScope::Clip => {
                let is_text = def.category == "text";
                let clip = if is_text {
                    json!({"start": 0.5, "duration": 4.0, "source": {"type": "text", "text": "Hello beat world", "size": 32}})
                } else {
                    json!({"start": 0.0, "duration": 6.0, "source": {"type": "media", "asset": "footage", "no_audio": true}, "fit": "cover"})
                };
                Some(e.mutate("clip", |p| ops::add_clip(p, None, ops::TrackTarget::New, clip)).unwrap())
            }
            PresetScope::Timeline => {
                match def.id.as_str() {
                    "lyrics_from_subtitles" => {
                        args.insert("asset".into(), json!("words"));
                    }
                    "auto_amv" => {
                        args.insert("title".into(), json!("TEST"));
                        args.insert("lyrics".into(), json!("words"));
                    }
                    "apply_to_all" => {
                        e.mutate("clip", |p| ops::add_clip(p, None, ops::TrackTarget::New, json!({"start": 0, "duration": 2, "source": {"type": "media", "asset": "footage"}}))).unwrap();
                    }
                    _ => {}
                }
                None
            }
        };
        let r = e.apply_preset(&def.id, target.as_deref(), &args, None);
        match r {
            Err(err) => failures.push(format!("{}: apply failed: {err}", def.id)),
            Ok(rep) => {
                let errs: Vec<String> = e.validate().into_iter().filter(|i| i.severity == Severity::Error).map(|i| format!("{:?} {}", i.object, i.message)).collect();
                if !errs.is_empty() {
                    failures.push(format!("{}: invalid project: {errs:?}", def.id));
                    continue;
                }
                if !rep.changed && !rep.logs.iter().any(|l| l.contains("no ")) {
                    failures.push(format!("{}: did nothing (logs {:?})", def.id, rep.logs));
                }
                for t in [1.0, 3.2] {
                    match e.render_frame(None, t, OutputMode::Straight) {
                        Ok(f) if f.warnings.is_empty() => {}
                        Ok(f) => failures.push(format!("{} @{t}: warnings {:?}", def.id, f.warnings)),
                        Err(err) => failures.push(format!("{} @{t}: render failed: {err}", def.id)),
                    }
                }
            }
        }
        count += 1;
    }
    println!("tested {count} presets");
    assert!(failures.is_empty(), "{} preset failures:\n{}", failures.len(), failures.join("\n"));
}
