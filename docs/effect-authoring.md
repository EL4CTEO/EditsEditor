```text
Custom effects (effect_create) are WGSL with a TOML header in //! lines:
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

```
