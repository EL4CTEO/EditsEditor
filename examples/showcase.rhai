// Showcase edit built only from generators, shapes and text — no footage needed.
// Run:  edits new examples/showcase.edits.json --width 1280 --height 720 --duration 12
//       edits script examples/showcase.edits.json examples/showcase.rhai
//       edits sheet examples/showcase.edits.json -o sheet.jpg
//
// A fixed 128 BPM beat grid stands in for analyzed music so every beat-synced
// preset/expression works without an audio file (normally: analyze_audio apply=true).

let bpm = 128.0;
let beat = 60.0 / bpm;
let comp = project.comp();
let dur = comp.duration;
let beats_list = [];
let t = 0.0;
while t < dur { beats_list.push(t); t += beat; }
project.update("main", #{ background: "#07030f" });
// timing (beats / downbeats / a drop at bar 3)
let downs = every(beats_list, 4);
let tm = #{ bpm: bpm, beats: beats_list, downbeats: downs, drops: [downs[3]],
            sections: [#{ start: 0.0, end: downs[3], label: "build", energy: 0.5 }, #{ start: downs[3], end: dur, label: "drop", energy: 1.0 }] };
project.set_project("timing", tm);

// --- background: animated gradient, switching to sunburst on the drop
let bg = project.add_track("background");
project.add_clip(bg, #{ start: 0.0, duration: downs[3], source: #{ type: "generator", effect: "gradient",
    params: #{ shape: "radial", color1: "#2b0a4f", color2: "#7a1fa2", color3: "#ff4e9b", stops: 3, offset: expr("t * 0.05") } } });
let burst = project.add_clip(bg, #{ start: downs[3], duration: dur - downs[3], source: #{ type: "generator", effect: "sunburst",
    params: #{ color_a: "#ff2e63", color_b: "#2b0a4f", rays: 20.0, speed: 0.08 } },
    transition_in: #{ effect: "flash_cut", duration: 0.35 } });
project.add_effect(burst, fx("vignette", #{ amount: 0.6 }));

// --- mid layer: kaleidoscope plasma "footage" stand-in, cut on every 2 beats with velocity-style punches
let mid = project.add_track("visuals");
let looks = ["plasma", "kaleido_pattern", "aurora", "caustics"];
let cuts = every(beats_list, 8);
for i in 0..cuts.len() {
    let a = cuts[i];
    let b = if i + 1 < cuts.len() { cuts[i + 1] } else { dur };
    let c = project.add_clip(mid, #{ start: a, duration: b - a, blend_mode: "screen", opacity: 0.55,
        source: #{ type: "generator", effect: looks[i % looks.len()] },
        transform: #{ scale: 0.62, position: [0.0, -30.0] } });
    project.set(c, "masks", [#{ shape: #{ type: "rect", size: [760.0, 430.0], radius: 36.0 }, feather: 6.0 }]);
    if i > 0 { project.set(c, "transition_in", #{ effect: choose(["zoom_through", "slice_slide", "glitch_transition", "spin"]), duration: 0.3 }); }
}

// --- shapes: a star that draws itself, a pulsing ring
let shp = project.add_track("shapes");
let star = project.add_clip(shp, #{ start: 0.5, duration: dur - 0.5, source: #{ type: "shape",
    shape: #{ type: "star", points: 5, outer_radius: 70.0, inner_radius: 30.0 },
    stroke: #{ color: "#fff6c9", width: 5.0 }, trim: #{ end: keys([[0.0, 0.0, "ease_in_out_cubic"], [1.5, 1.0]]) } },
    transform: #{ position: [-470.0, -220.0], rotation: expr("t * 40.0") } });
project.add_effect(star, fx("deep_glow", #{ intensity: 2.0, threshold: 0.1, tint: "#ffd36e" }));
let ring = project.add_clip(shp, #{ start: 0.0, duration: dur, source: #{ type: "shape",
    shape: #{ type: "ring", outer_radius: 300.0, inner_radius: 292.0 }, fill: "#ffffff" },
    opacity: 0.5, transform: #{ scale: expr("1.0 + 0.12 * pulse(9.0)"), position: [0.0, -30.0] } });

// --- overlays: speed lines on the drop, sparkles throughout
let ov = project.add_track("overlays");
project.add_clip(ov, #{ start: downs[3], duration: beat * 8.0, blend_mode: "add", opacity: 0.8,
    source: #{ type: "generator", effect: "speed_lines", params: #{ count: 120.0, inner_radius: 0.35 } } });
project.add_clip(ov, #{ start: 0.0, duration: dur, source: #{ type: "generator", effect: "sparkles", params: #{ density: 1.5, rainbow: 0.4 } } });

// --- title + kinetic subtitle
let tx = project.add_track("text");
let title = project.add_clip(tx, #{ start: 0.2, duration: dur - 0.2, source: #{ type: "text", text: "EDITS EDITOR", size: 120.0, weight: 900,
    gradient: ["#ffffff", "#ffb7f0"], stroke: #{ color: "#2b0a4f", width: 8.0 }, shadow: #{ color: "#ff2e9a88", offset: [0.0, 8.0], blur: 18.0 } },
    transform: #{ position: [0.0, 230.0] } });
project.apply_preset(title, "text_pop", #{ exit: false });
project.add_effect(title, fx("transform", #{ scale: expr("1.0 + 0.06 * pulse(10.0)") }));
let sub = project.add_clip(tx, #{ start: 1.0, duration: dur - 1.0, source: #{ type: "text", text: "made by AI agents · 169 effects · 103 presets", size: 34.0, weight: 500, color: "#fff6c9" },
    transform: #{ position: [0.0, 310.0] } });
project.apply_preset(sub, "text_slide_up", #{});

// --- global finishing: impact on the drop, glow, grade, grain
let fxt = project.add_track("finishing");
let adj = project.add_clip(fxt, #{ start: 0.0, duration: dur, source: #{ type: "adjustment" } });
let d = downs[3];
project.add_effect(adj, fx("transform", #{ scale: hit_keys(d, 1.0, 1.25, 0.45, "ease_out_expo"), edge: "mirror" }));
project.add_effect(adj, fx("shockwave", #{ radius: keys([[d, 0.0, "ease_out_cubic"], [d + 0.8, 1.6]]), strength: keys([[d, 0.07], [d + 0.8, 0.0]]) }));
project.add_effect(adj, fx("rgb_split", #{ amount: pulse_keys(every(beats_list, 2), 0.0, 14.0, 0.0, 0.2, "ease_out_expo") }));
project.add_effect(adj, fx("flash", #{ amount: hit_keys(d, 0.0, 0.9, 0.3, "ease_out_quad") }));
project.add_effect(adj, fx("bloom", #{ intensity: 0.5 }));
project.add_effect(adj, fx("film_grain", #{ amount: 0.05 }));
log("showcase built: " + project.clips().len() + " clips");
