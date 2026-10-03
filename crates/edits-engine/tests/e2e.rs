use std::path::Path;

use edits_core::{Project, ops};
use edits_engine::{AnalysisOptions, Engine, ExportSettings, ImportOptions, OutputMode};
use serde_json::json;

fn ffmpeg_ok() -> bool {
    edits_media::Ffmpeg::locate().is_ok()
}

fn make_media(dir: &Path) {
    let ff = edits_media::Ffmpeg::locate().unwrap();
    // 4s test pattern video with audio
    let st = ff
        .cmd()
        .args(["-y", "-f", "lavfi", "-i", "testsrc2=s=320x180:r=24:d=4", "-f", "lavfi", "-i", "sine=frequency=330:duration=4"])
        .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest"])
        .arg(dir.join("clip.mp4"))
        .status()
        .unwrap();
    assert!(st.success());
    // 8s 120bpm click track
    let st = ff
        .cmd()
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc='0.9*sin(2*PI*60*t)*exp(-30*mod(t,0.5))+0.4*random(0)*exp(-60*mod(t,0.5))':s=48000:d=8",
        ])
        .arg(dir.join("music.wav"))
        .status()
        .unwrap();
    assert!(st.success());
    // a png
    image::RgbaImage::from_fn(64, 64, |x, y| image::Rgba([(x * 4) as u8, (y * 4) as u8, 200, if (x + y) % 16 < 12 { 255 } else { 0 }]))
        .save(dir.join("logo.png"))
        .unwrap();
    std::fs::write(dir.join("lyrics.lrc"), "[00:00.50]Hello world\n[00:02.00]Second line\n").unwrap();
}

#[test]
fn full_pipeline() {
    if !ffmpeg_ok() {
        eprintln!("ffmpeg missing; skipping");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    make_media(dir.path());
    let proj_path = dir.path().join("test.edits.json");
    let mut e = Engine::create(&proj_path, Project::new("e2e", 320, 180, 24.0, 4.0)).unwrap();
    if e.gpu_info().is_err() {
        eprintln!("no GPU; skipping");
        return;
    }
    let imported = e.import(&[dir.path().to_string_lossy().to_string()], &ImportOptions::default()).unwrap();
    let ids: Vec<&str> = imported.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"clip") && ids.contains(&"music") && ids.contains(&"logo") && ids.contains(&"lyrics"), "{ids:?}");

    // music on its own track + analysis
    e.mutate("music", |p| {
        ops::add_clip(p, None, ops::TrackTarget::New, json!({"name": "music", "start": 0, "source": {"type": "media", "asset": "music"}}))
    })
    .unwrap();
    let an = e.analyze_audio("music", &AnalysisOptions::default(), true, None).unwrap();
    assert!((an.bpm - 120.0).abs() < 3.0, "bpm {}", an.bpm);
    assert!(!e.project.timing.beats.is_empty());

    // footage with effects, cut in two with a transition
    let c1 = e
        .mutate("footage", |p| {
            ops::add_clip(
                p,
                None,
                ops::TrackTarget::New,
                json!({
                    "start": 0, "duration": 4, "source": {"type": "media", "asset": "clip"},
                    "fit": "cover", "motion_blur": true,
                    "transform": {"scale": {"keyframes": [[0, 1.3], [0.4, 1.0, "punch"]], "expr": "value * (1.0 + 0.05 * pulse(10.0))"}},
                    "effects": [
                        {"effect": "deep_glow", "params": {"intensity": 1.5}},
                        {"effect": "rgb_split", "params": {"amount": {"expr": "20.0 * pulse(12.0)"}}},
                        {"effect": "shake", "params": {"amplitude": 8}, "mix": 0.8}
                    ]
                }),
            )
        })
        .unwrap();
    let c2 = e.mutate("split", |p| ops::split_clip(p, &c1, 2.0)).unwrap();
    e.mutate("transition", |p| {
        ops::set_path(p, &c2, "transition_in", json!({"effect": "zoom_through", "duration": 0.5, "ease": "ease_in_out"}))
    })
    .unwrap();
    e.mutate("mask", |p| ops::set_path(p, &c2, "masks", json!([{"shape": {"type": "ellipse", "size": [260, 160]}, "feather": 20}])))
        .unwrap();
    e.mutate("echo", |p| ops::set_path(p, &c1, "echo", json!({"count": 3, "interval": 0.05}))).unwrap();

    // text with animator, shape, generator overlay, logo with matte, adjustment layer
    e.mutate("overlays", |p| {
        ops::add_clip(p, None, ops::TrackTarget::New, json!({
            "start": 0.2, "duration": 3, "source": {"type": "text", "text": "EDITS ✦ 編集", "size": 36,
                "stroke": {"color": "#000", "width": 3}, "shadow": {"blur": 6},
                "animator": {"from": {"opacity": 0, "offset": [0, 30], "scale": 1.6}, "stagger": 0.05, "out": {"to": {"opacity": 0}}}},
            "transform": {"position": [0, 50]}
        }))?;
        ops::add_clip(p, None, ops::TrackTarget::New, json!({
            "start": 0, "duration": 4, "source": {"type": "shape", "shape": {"type": "star", "points": 5, "outer_radius": 30, "inner_radius": 12},
                "fill": {"kind": "radial", "stops": [[0, "#fff"], [1, "#ff2e88"]]}, "stroke": {"color": "white", "width": 2},
                "trim": {"end": {"keyframes": [[0, 0], [1, 1]]}}},
            "transform": {"position": [-110, -50], "rotation": {"expr": "t * 90.0"}}
        }))?;
        ops::add_clip(p, None, ops::TrackTarget::New, json!({"start": 1, "duration": 2, "blend_mode": "screen", "opacity": 0.7,
            "source": {"type": "generator", "effect": "speed_lines", "params": {"count": 60}}}))?;
        let logo = ops::add_clip(p, None, ops::TrackTarget::New, json!({"start": 0, "duration": 4, "source": {"type": "media", "asset": "logo"},
            "fit": "none", "transform": {"position": [120, -55], "rotation_y": 30}, "blend_mode": "add"}))?;
        let _ = logo;
        ops::add_clip(p, None, ops::TrackTarget::New, json!({"start": 3, "duration": 1, "source": {"type": "adjustment"},
            "effects": [{"effect": "color_grade", "params": {"look": "cyberpunk"}}, {"effect": "vignette"}]}))?;
        Ok(())
    }).unwrap();

    // nested comp
    e.mutate("nested", |p| {
        let mut inner = edits_core::Composition::new("inner", 160, 90, 24.0, 2.0);
        inner.background = edits_core::Color([0.1, 0.0, 0.2, 1.0]);
        ops::add_comp(p, Some("inner"), inner)?;
        ops::add_clip(
            p,
            Some("inner"),
            ops::TrackTarget::New,
            json!({"start": 0, "duration": 2, "source": {"type": "generator", "effect": "plasma"}}),
        )?;
        ops::add_clip(
            p,
            None,
            ops::TrackTarget::New,
            json!({"start": 2.5, "duration": 1.5, "source": {"type": "comp", "comp": "inner"},
            "fit": "none", "transform": {"scale": 0.6, "position": [100, 40]}, "end_behavior": "loop"}),
        )?;
        Ok(())
    })
    .unwrap();

    let issues = e.validate();
    let errors: Vec<_> = issues.iter().filter(|i| i.severity == edits_core::validate::Severity::Error).collect();
    assert!(errors.is_empty(), "{errors:#?}");

    for t in [0.1, 1.0, 2.1, 2.4, 3.5] {
        let r = e.render_frame(None, t, OutputMode::Straight).unwrap();
        assert!(r.warnings.is_empty(), "t={t}: {:?}", r.warnings);
        assert_eq!(r.frame.width, 320);
        let lit = r.frame.data.chunks_exact(4).filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 30).count();
        assert!(lit > 1000, "frame at {t} mostly black ({lit})");
        if t == 2.4 {
            r.frame.to_png().map(|png| std::fs::write(dir.path().join("frame.png"), png)).unwrap().unwrap();
        }
    }
    let (sheet, _) = e.contact_sheet(None, &[0.5, 1.5, 2.5, 3.5], 2, 160).unwrap();
    assert!(sheet.width > 300);
    let wf = e.waveform_image("music", 0.0, None, 400, 100).unwrap();
    assert_eq!(wf.width, 400);

    // presets via scripting
    let script = r#"
        let tr = project.add_track("lyrics");
        let cues = subtitles("lyrics");
        for c in cues {
            project.add_clip(tr, #{ start: c.start, duration: c.end - c.start, source: #{ type: "text", text: c.text, size: 24.0 } });
        }
        cues.len()
    "#;
    let (rep, _) = e.run_script(script, json!(null), false).unwrap();
    assert_eq!(rep.result, json!(2));

    // export
    let out = dir.path().join("out.mp4");
    let mut s = ExportSettings::new(out.to_string_lossy());
    s.range = Some([1.5, 2.5]);
    let rep = e.export(None, &s, None, |_| {}).unwrap();
    assert_eq!(rep.frames, 24);
    assert!(rep.audio);
    assert!(out.exists());
    println!("export: {rep:?}");
    // undo works and file is saved
    assert!(e.undo().unwrap().is_some());
    let reopened = Engine::open(&proj_path).unwrap();
    assert_eq!(reopened.project, e.project);
}
