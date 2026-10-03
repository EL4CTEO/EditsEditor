//! id = "fast_blur"
//! name = "Fast Blur"
//! kind = "filter"
//! category = "blur"
//! description = "Very cheap large-radius blur (dual Kawase, downsampled). Great for backgrounds and huge radii."
//! tags = ["blur", "fast", "background"]
//! params = [
//!   { name = "radius", type = "float", default = 30.0, min = 0.0, max = 500.0, desc = "Approximate blur radius in pixels" },
//! ]
//! passes = [
//!   { entry = "down", scale = 0.5 },
//!   { entry = "down", scale = 0.25 },
//!   { entry = "down", scale = 0.125 },
//!   { entry = "up", scale = 0.25 },
//!   { entry = "up", scale = 0.5 },
//!   { entry = "compose", inputs = ["prev", "input"] },
//! ]
fn offs(p: Params) -> f32 { return clamp(p.radius / 40.0, 0.0, 6.0) + 0.5; }
fn down(uv: vec2f, p: Params) -> vec4f {
    let o = texel() * offs(p);
    var c = src_clamp(uv) * 4.0;
    c += src_clamp(uv + vec2f(-o.x, -o.y));
    c += src_clamp(uv + vec2f(o.x, -o.y));
    c += src_clamp(uv + vec2f(-o.x, o.y));
    c += src_clamp(uv + vec2f(o.x, o.y));
    return c / 8.0;
}
fn up(uv: vec2f, p: Params) -> vec4f {
    let o = texel() * offs(p);
    var c = src_clamp(uv + vec2f(-o.x * 2.0, 0.0));
    c += src_clamp(uv + vec2f(-o.x, o.y)) * 2.0;
    c += src_clamp(uv + vec2f(0.0, o.y * 2.0));
    c += src_clamp(uv + vec2f(o.x, o.y)) * 2.0;
    c += src_clamp(uv + vec2f(o.x * 2.0, 0.0));
    c += src_clamp(uv + vec2f(o.x, -o.y)) * 2.0;
    c += src_clamp(uv + vec2f(0.0, -o.y * 2.0));
    c += src_clamp(uv + vec2f(-o.x, -o.y)) * 2.0;
    return c / 12.0;
}
fn compose(uv: vec2f, p: Params) -> vec4f {
    let k = clamp(p.radius / 8.0, 0.0, 1.0);
    return mix(orig(uv), src_clamp(uv), k);
}
