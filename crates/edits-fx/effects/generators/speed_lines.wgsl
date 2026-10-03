//! id = "speed_lines"
//! name = "Speed Lines (Focus Lines)"
//! kind = "generator"
//! category = "anime"
//! description = "Anime radial speed/focus lines converging on a point. Flickers like hand-drawn manga lines."
//! tags = ["speed lines", "focus lines", "anime", "manga", "impact", "amv"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "count", type = "float", default = 90.0, min = 8.0, max = 400.0, desc = "Number of lines" },
//!   { name = "thickness", type = "float", default = 0.35, min = 0.01, max = 1.0 },
//!   { name = "inner_radius", type = "float", default = 0.25, min = 0.0, max = 1.5, desc = "Clear area around the center" },
//!   { name = "softness", type = "float", default = 0.2, min = 0.0, max = 1.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "fps", type = "float", default = 12.0, min = 0.0, max = 60.0, desc = "Redraw rate (anime style is 8-12)" },
//!   { name = "opacity", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let a = atan2(d.y, d.x) / TAU + 0.5;
    let r = length(d);
    let f = floor(time() * p.fps) + seed() * 13.0;
    let id = floor(a * p.count);
    let rnd = hash12(vec2f(id, f));
    let w = fract(a * p.count);
    let thick = p.thickness * (0.3 + rnd);
    let line_mask = 1.0 - smoothstep(thick * 0.5, thick * 0.5 + 0.1, abs(w - 0.5));
    let start = p.inner_radius * (0.8 + hash12(vec2f(id * 3.1, f)) * 0.6);
    let radial = smoothstep(start, start + p.softness + 0.001, r);
    let present = step(0.35, rnd);
    let v = line_mask * radial * present * p.opacity * p.color.a;
    return vec4f(p.color.rgb * v, v);
}
