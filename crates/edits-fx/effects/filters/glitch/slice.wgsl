//! id = "slice"
//! name = "Slice Shift"
//! kind = "filter"
//! category = "glitch"
//! description = "Horizontal (or angled) slices pushed in alternating directions. Clean, stylish glitch for impacts."
//! tags = ["slice", "glitch", "cut", "amv", "impact"]
//! params = [
//!   { name = "amount", type = "float", default = 60.0, min = 0.0, max = 1000.0, desc = "Max shift in pixels" },
//!   { name = "slices", type = "float", default = 12.0, min = 1.0, max = 200.0 },
//!   { name = "angle", type = "angle", default = 0.0 },
//!   { name = "random", type = "float", default = 1.0, min = 0.0, max = 1.0, desc = "0 = alternating, 1 = random amounts" },
//!   { name = "edge", type = "enum", options = ["transparent", "clamp", "mirror", "wrap"], default = "wrap" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let dir = dir_deg(p.angle);
    let perp = vec2f(-dir.y, dir.x);
    let coord = dot(uv - 0.5, perp * vec2f(1.0, 1.0 / aspect())) + 0.5;
    let s = floor(coord * p.slices);
    let alt = select(-1.0, 1.0, fmod_pos(s, 2.0) < 1.0);
    let rnd = hash11(s + seed() * 31.0) * 2.0 - 1.0;
    let k = mix(alt, rnd, p.random);
    return src_edge(uv + dir * k * p.amount * texel(), p.edge);
}
