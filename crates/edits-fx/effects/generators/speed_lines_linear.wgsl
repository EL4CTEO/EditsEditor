//! id = "speed_lines_linear"
//! name = "Linear Speed Lines"
//! kind = "generator"
//! category = "anime"
//! description = "Parallel action lines streaking across the frame (dash, chase, sprint scenes)."
//! tags = ["speed lines", "action", "anime", "motion", "dash"]
//! params = [
//!   { name = "angle", type = "angle", default = 0.0 },
//!   { name = "density", type = "float", default = 80.0, min = 4.0, max = 400.0 },
//!   { name = "speed", type = "float", default = 3.0, min = 0.0, max = 30.0 },
//!   { name = "length", type = "float", default = 0.4, min = 0.02, max = 2.0 },
//!   { name = "thickness", type = "float", default = 0.25, min = 0.01, max = 1.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "opacity", type = "float", default = 0.8, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle);
    let perp = vec2f(-d.y, d.x);
    let q = vec2f(dot((uv - 0.5) * vec2f(aspect(), 1.0), d), dot((uv - 0.5) * vec2f(aspect(), 1.0), perp));
    let row = floor(q.y * p.density);
    let rnd = hash11(row + seed() * 7.0);
    let fy = fract(q.y * p.density);
    let x = q.x * 0.5 - time() * p.speed * (0.5 + rnd) + rnd * 10.0;
    let seg = fract(x / (p.length * 2.0 + rnd));
    let on = smoothstep(0.0, 0.1, seg) * (1.0 - smoothstep(p.length * 0.5, p.length * 0.5 + 0.1, seg));
    let th = 1.0 - smoothstep(p.thickness * 0.5 * rnd, p.thickness * 0.5 * rnd + 0.15, abs(fy - 0.5));
    let v = on * th * step(0.5, rnd) * p.opacity * p.color.a;
    return vec4f(p.color.rgb * v, v);
}
