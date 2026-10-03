//! id = "spotlight"
//! name = "Spotlight"
//! kind = "filter"
//! category = "stylize"
//! description = "Darken everything except a soft circle (focus on a character)."
//! tags = ["spotlight", "focus", "highlight", "vignette"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "radius", type = "float", default = 0.25, min = 0.0, max = 2.0 },
//!   { name = "softness", type = "float", default = 0.15, min = 0.0, max = 1.0 },
//!   { name = "darkness", type = "float", default = 0.8, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let r = length(centered(uv, p.center));
    let k = smoothstep(p.radius, p.radius + p.softness, r) * p.darkness;
    return vec4f(c.rgb * (1.0 - k), c.a);
}
