//! id = "channel_mixer"
//! name = "Channel Mixer"
//! kind = "filter"
//! category = "color"
//! description = "Rebuild each output channel from a weighted mix of the input channels."
//! tags = ["color", "channels", "mixer", "swap"]
//! params = [
//!   { name = "red", type = "vec3", default = [1.0, 0.0, 0.0], desc = "Output red = dot(rgb, red)" },
//!   { name = "green", type = "vec3", default = [0.0, 1.0, 0.0] },
//!   { name = "blue", type = "vec3", default = [0.0, 0.0, 1.0] },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = unpremul(src_clamp(uv));
    let rgb = vec3f(dot(c.rgb, p.red), dot(c.rgb, p.green), dot(c.rgb, p.blue));
    return premul(vec4f(max(rgb, vec3f(0.0)), c.a));
}
