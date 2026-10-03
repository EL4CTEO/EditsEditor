//! id = "slice_slide"
//! name = "Slice Slide"
//! kind = "transition"
//! category = "motion"
//! description = "Horizontal slices slide in alternating directions to reveal the next clip — punchy AMV cut."
//! tags = ["slice", "slide", "stripes", "amv", "glitch"]
//! params = [
//!   { name = "slices", type = "float", default = 8.0, min = 2.0, max = 60.0 },
//!   { name = "angle", type = "angle", default = 0.0 },
//!   { name = "stagger", type = "float", default = 0.3, min = 0.0, max = 0.9 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle);
    let perp = vec2f(-d.y, d.x);
    let s = floor((dot(uv - 0.5, perp) + 0.5) * p.slices);
    let alt = select(-1.0, 1.0, fmod_pos(s, 2.0) < 1.0);
    let delay = hash11(s + seed()) * p.stagger;
    let t = clamp((progress() - delay) / (1.0 - p.stagger), 0.0, 1.0);
    let e = t * t * (3.0 - 2.0 * t);
    let a = src(uv - d * alt * e);
    let b = orig(uv + d * alt * (1.0 - e));
    return over(b, a);
}
