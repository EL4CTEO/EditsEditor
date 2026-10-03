//! Built-in documentation served to agents (`reference` tool) and printed by the CLI.

pub const INSTRUCTIONS: &str = r##"EditsEditor: a GPU video editor you drive through tools — built for AMVs, anime edits, MVs and anything motion-graphics.

Typical workflow:
1. project_open {path, create:true, width, height, fps, duration}
2. media_import {paths:[folder or files]}  (video, audio, images, gif/webp/apng, svg, image sequences, fonts, .cube LUTs, .lrc/.srt lyrics)
3. Put the song on a track (clip_add), then analyze_audio {asset, apply:true} → beats, downbeats, drops, sections land in project timing.
4. detect_scenes / media_preview to look at your footage; pick shots (high motion for drops).
5. Build the edit: clip_add (cuts on beats), preset_apply (impacts, velocity, looks, text), effect_add, transitions (set transition_in),
   or write a script_run that loops over beats() and builds everything at once. preset_apply {preset:"auto_amv"} gives a full first pass.
6. LOOK at your work: render_frames {count:8} (contact sheet) and render_frame {time}. Iterate.
7. validate, then export {output:"out.mp4"}.

Key facts:
- Times are seconds. Keyframe times are clip-local (0 = clip start). Positions are pixels offset from the composition center (+y down).
- Any animatable property accepts: a value, {"keyframes":[[t,v,"ease"],...]}, and/or {"expr":"value * (1.0 + 0.1*pulse(8.0))"}.
- Expressions can read beats/audio: pulse(), bass(), beat_phase(), wiggle()... (reference topic "expressions").
- Effects are WGSL shaders (effects_list / effect_info). Write your own with effect_create. Presets are Rhai scripts (presets_list / preset_info); write your own with preset_create.
- Every mutation is undoable (history) and autosaved to the project file; the viewer app (edits view) live-updates.
- Use project_summary often — it is the cheapest way to see the whole timeline.
Read `reference` topics: workflow, format, expressions, scripting, effects, presets, easings, blend_modes, tips.
"##;

pub const FORMAT: &str = r##"Project format quick reference (full JSON schema: tool `schema`).
Project { root: "main", compositions: {id: Composition}, assets: {id: Asset}, timing, custom_effects, custom_presets, variables, script_library }
Composition { width, height, fps, duration, background, tracks: [Track] (bottom→top), effects (comp-level), markers, motion_blur {samples, shutter_angle}, volume }
Track { id, name, enabled, muted, solo, opacity, blend_mode, volume, effects (applied to the track result), clips }
Clip {
  id, start, duration, source,
  in (source in-point s), speed (number or keyframes: velocity ramps), time_remap, reverse, freeze (source time), end_behavior: hold|loop|ping_pong|transparent,
  transform { position [x,y] px from center, anchor, scale (n or [x,y]), rotation°, rotation_x°, rotation_y° (3D), z, skew°, perspective, flip_x, flip_y },
  fit: contain|cover|stretch|none|width|height, crop [l,t,r,b] 0..1, opacity, blend_mode,
  effects [EffectInstance], masks [Mask], matte {clip, mode: alpha|alpha_inverted|luma|luma_inverted}, hidden (use as matte only),
  transition_in { effect, duration, ease, params, centered }, motion_blur: bool, echo { count, interval, decay, blend, behind },
  audio { volume, pan, mute, fade_in, fade_out, effects: [lowpass|highpass {cutoff,q} | echo | distortion | bitcrush | stutter {start,end,slice} | gain {db}] }
}
Sources (source.type):
  media {asset, no_audio, no_video} · solid {color, size?} · text {text, font, weight, italic, size, color, align, line_height, letter_spacing,
  max_width, stroke {color,width}, shadow {color,offset,blur}, background {color,padding,radius}, gradient [top,bottom], uppercase,
  animator {unit: char|word|line, stagger, duration, start, ease, order: forward|backward|center_out|edges_in|random,
            from {opacity, offset, scale, rotation}, out {stagger,duration,ease,to}, wave [amp,freq,phase], jitter, jitter_rate}}
  · shape {shape: rect{size,radius}|ellipse{size}|polygon{sides,radius}|star{points,outer_radius,inner_radius}|line{from,to}|ring|path{d},
           fill (color or {kind: linear|radial, stops:[[o,color]], angle}), stroke {color,width,dash,cap}, trim {start,end,offset}}
  · generator {effect, params} (procedural: speed_lines, sakura, particles, gradients...) · comp {comp} (nested) · adjustment (effects apply to everything below)
EffectInstance { id, effect, params {name: value|keyframes|expr}, mix 0..1, enabled, range [start,end] }
Mask { shape: rect{center,size,radius}|ellipse{center,size}|polygon{points}|path{d}, mode: add|subtract|intersect|difference, feather, expansion, opacity, invert, offset, rotation, scale }
Colors: "#rrggbb[aa]", CSS names, "rgb()/hsl()", or [r,g,b,a] 0..1. Easings: see topic "easings" (names, {bezier:[..]}, {spring:{..}}, {steps:n}, {back:s}).
Ids are short ("c12", "fx3"); you may set your own ids when adding.
Example clip:
{"start":2.0,"duration":1.5,"source":{"type":"media","asset":"op_footage"},"in":34.2,"fit":"cover",
 "transform":{"scale":{"keyframes":[[0,1.3],[0.3,1.0,"punch"]]}},
 "effects":[{"effect":"deep_glow","params":{"intensity":1.4}},{"effect":"rgb_split","params":{"amount":{"expr":"25.0*pulse(10.0)"}}}],
 "transition_in":{"effect":"flash_cut","duration":0.25}}
"##;

pub const SCRIPTING: &str = r##"Scripts & presets use Rhai (Rust-like syntax: let, fn, if, for x in arr, for i in 0..n, #{ map }, [array]).
script_run executes code with:
  project — live handle:
    add_track(name[, comp]) → id; add_clip(track_or_"", #{clip}) → id; add_clip_in(comp, track, #{clip}); add_comp(id, w, h, fps, dur)
    add_effect(target_id, fx(...)) → id; update(id, #{merge patch}); set(id, "path.to.field", value); get(id, path); unset(id, path)
    keyframe(id, path, t, v[, ease]); expr(id, path, "expression" or ""); remove(id); split(id, t) → new id; move_clip(id, start[, track])
    duplicate(id, start) → id; add_marker(t, label); set_duration(s); set_var(name, v); apply_preset(target_or_"", preset, #{args})
    clip(id) / object(id) → map; tracks() → ids; clips(track) / clips() → ids; assets([kind]) → ids; asset(id); asset_duration(id); comp(); json(); new_id(prefix)
  timing: beats(), downbeats(), drops(), accents(), sections() (maps start/end/label/energy), bpm(), beats_between(a,b), nearest_beat(t)
  media: scenes(asset) → [#{start,end,motion,brightness,saturation,color,thumb}], subtitles(asset) → [#{start,end,text,words}]
  builders: fx(id[, params]), keys([[t,v,ease],...]), expr(code[, base]), hit_keys(at, base, peak, dur[, ease]),
            pulse_keys(times, base, peak, [attack,] release[, ease]), pick_times(ctx, "beats|downbeats|drops|half|bars2|accents", every), every(arr,n), dir_vec(dir, dist)
  map helpers (on clip maps): clip.add_fx(fx(..)), clip.set_kf(path, t, v, ease), clip.animate(path, keys), clip.set_path(path, v), clip.get_path(path), clip.merge(#{..})
  random (deterministic): rand(), rand(a,b), rand_int(a,b), choose(arr), shuffle(arr) · math: lerp, clamp, smoothstep, remap, hsv(h,s,v), ease_value(name,x) · log(x) / print(x)
  args (map) and ctx (fps, width, height, comp_duration, bpm, beat_len, beats/downbeats/drops/accents; clip presets also clip_id, clip_start, clip_duration and clip-local beats)
Clip presets additionally get `clip` (map) — edit it directly: clip.opacity = 0.5; clip.add_fx(fx("glow")); clip.transform = #{ scale: 1.2 }.
Preset file format: lines starting with //! form a TOML header:
//! id = "my_preset"
//! scope = "clip"          # or "timeline"
//! category = "impact"
//! description = "..."
//! params = [ { name = "at", type = "float", default = 0.0, desc = "..." } ]   # types: float int bool color enum(options=[..]) string vec2 point
<rhai code>
Example script: cut every 2 beats between two shots and punch on downbeats:
  let tr = project.add_track("cuts"); let bs = every(beats(), 2);
  for i in 0..bs.len()-1 { let c = project.add_clip(tr, #{start: bs[i], duration: bs[i+1]-bs[i], source: #{type:"media", asset: if i % 2 == 0 {"shotA"} else {"shotB"}}, fit:"cover"});
    project.apply_preset(c, "zoom_punch_beats", #{on: "downbeats"}); }
"##;

pub const EFFECT_AUTHORING: &str = r##"Custom effects (effect_create) are WGSL with a TOML header in //! lines:
//! id = "my_effect"
//! kind = "filter"            # filter | transition | generator
//! category = "stylize"
//! description = "..."
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 10.0, desc = "..." },
//!   { name = "tint", type = "color", default = "#ff00aa" },
//!   { name = "mode", type = "enum", options = ["a", "b"], default = "a" },   # passed as f32 index
//! ]
//! # optional multi-pass: passes = [ { entry = "blur_h", scale = 0.5 }, { entry = "blur_v", scale = 0.5 }, { entry = "combine", inputs = ["prev", "input"] } ]
fn effect(uv: vec2f, p: Params) -> vec4f { return src(uv) * vec4f(p.tint.rgb, 1.0) * p.amount; }
Param types: float int bool angle(deg) vec2 vec3 point(uv 0..1) color(linear rgb + straight alpha) enum image lut. Access as p.<name>.
Colors are linear premultiplied RGBA. uv (0,0)=top-left.
Prelude helpers: src(uv) (transparent outside), src_clamp/src_wrap/src_mirror/src_edge(uv,mode), orig(uv) (effect input / transition 'to'),
  from_img/to_img (transitions), extra(uv) (image/LUT param), res(), texel(), aspect(), time(), ltime(), progress(), seed(), bass(), treble(),
  beat_pulse(decay), luma, rgb2hsv, hsv2rgb, unpremul, premul, to_srgb, to_linear, grade_begin/grade_end (perceptual grading), over(a,b),
  rot2(a), uv_transform(uv,center,angle,scale), centered(uv,center), dir_deg(deg), hash11/12/22/33, vnoise, gnoise, fbm(p,oct), worley, noise1(t),
  sd_box, sd_circle, aa_step, blur_dir(uv, dir, radius), fmod_pos, screen3, with_mix. Globals: U.g.time, U.g.local_time, U.g.progress,
  U.g.beat_time, U.g.bpm, U.g.audio_bands, U.g.pass_data, U.g.resolution.
Errors from effect_create include exact line numbers of your file.
"##;

pub const TIPS: &str = r##"AMV craft tips for agents:
- Cut on beats; go faster (1 beat) in drops/choruses, slower (2-4 beats) in verses; match high-motion shots to high energy (detect_scenes motion).
- Hit the downbeats: impact / zoom_punch / flash_hit / shake_hit at the exact beat time (clip-local!). pulse_keys(times...) builds these.
- Velocity edits (velocity_ramp, velocity_beats) + motion_blur:true sell speed. Echo (clip.echo) gives afterimage trails.
- Use transition_in sparingly: flash_cut on drops, zoom_through / whip_pan for energy, crossfade/dreamy for calm parts.
- Put global looks on an adjustment clip (global_grade) instead of every clip. Overlays (sakura, speed_lines) go on top tracks with blend modes (screen/add).
- Text: lyrics_from_subtitles, text_* presets; stroke + shadow keeps text readable.
- Expressions keep things in sync automatically: scale {"expr":"value*(1.0+0.08*pulse(9.0))"}, rotation {"expr":"wiggle(2.0,1.5)"}.
- Always check: render_frames {count: 12} after big changes; render_frame at exact hit times; validate before export.
- Agents can create entirely new looks: effect_create for shaders, preset_create for reusable recipes, script_library for shared Rhai functions.
"##;

pub fn topic(name: &str) -> Option<String> {
    Some(match name {
        "overview" | "workflow" | "" => INSTRUCTIONS.to_string(),
        "format" | "project" => FORMAT.to_string(),
        "scripting" | "scripts" | "rhai" => SCRIPTING.to_string(),
        "expressions" | "expr" => edits_engine::expr::EXPR_REFERENCE.to_string(),
        "effects" | "wgsl" | "shaders" => EFFECT_AUTHORING.to_string(),
        "presets" => format!("{SCRIPTING}\nUse presets_list / preset_info to browse; preset_create to add your own."),
        "easings" | "easing" => format!(
            "Easing names: {}\nObject forms: {{\"bezier\":[x1,y1,x2,y2]}}, {{\"steps\":n}}, {{\"back\":overshoot}}, {{\"spring\":{{\"stiffness\":170,\"damping\":12,\"mass\":1}}}}, {{\"elastic\":{{\"amplitude\":1,\"period\":0.3}}}}.\nThe ease on a keyframe shapes the segment to the NEXT keyframe. AMV favorites: punch (zoom hits), whip (velocity), snap, glide, ease_out_expo, ease_out_back.",
            edits_core::Easing::NAMES.join(", ")
        ),
        "blend_modes" | "blend" => format!(
            "Blend modes: {}",
            edits_core::BlendMode::ALL.iter().map(|b| b.name()).collect::<Vec<_>>().join(", ")
        ),
        "tips" | "craft" => TIPS.to_string(),
        _ => return None,
    })
}

pub const TOPICS: &[&str] = &["workflow", "format", "expressions", "scripting", "effects", "presets", "easings", "blend_modes", "tips"];
