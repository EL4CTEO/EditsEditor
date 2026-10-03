//! id = "checker"
//! name = "Checkerboard / Grid Pattern"
//! kind = "generator"
//! category = "background"
//! description = "Scrolling checkerboard or grid lines (retro, Y2K, title cards)."
//! tags = ["checker", "grid", "pattern", "y2k", "retro"]
//! params = [
//!   { name = "pattern", type = "enum", options = ["checker", "grid", "dots", "stripes"], default = "checker" },
//!   { name = "size", type = "float", default = 80.0, min = 2.0, max = 1000.0, desc = "Cell size in pixels" },
//!   { name = "color_a", type = "color", default = "#111111" },
//!   { name = "color_b", type = "color", default = "#ff4fa3" },
//!   { name = "scroll", type = "vec2", default = [40.0, 20.0], desc = "Pixels per second" },
//!   { name = "angle", type = "angle", default = 0.0 },
//!   { name = "line_width", type = "float", default = 0.08, min = 0.01, max = 0.5 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var px = rot2(radians(p.angle)) * ((uv - 0.5) * res()) - p.scroll * time();
    let q = px / p.size;
    let f = fract(q);
    let m = i32(p.pattern);
    var k = 0.0;
    if (m == 0) { k = fmod_pos(floor(q.x) + floor(q.y), 2.0); }
    else if (m == 1) { k = select(0.0, 1.0, min(f.x, f.y) < p.line_width); }
    else if (m == 2) { k = 1.0 - smoothstep(p.line_width * 3.0, p.line_width * 3.0 + 0.05, length(f - 0.5)); }
    else { k = step(0.5, fract(q.x + q.y * 0.0)); }
    return vec4f(mix(p.color_a.rgb, p.color_b.rgb, k), 1.0);
}
