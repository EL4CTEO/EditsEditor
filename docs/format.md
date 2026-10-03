```text
Project format quick reference (full JSON schema: tool `schema`).
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

```
