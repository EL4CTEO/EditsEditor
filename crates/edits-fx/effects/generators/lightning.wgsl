//! id = "lightning"
//! name = "Lightning Bolt"
//! kind = "generator"
//! category = "energy"
//! description = "Flickering branching electric bolt between two points."
//! tags = ["lightning", "electric", "thunder", "energy", "power"]
//! params = [
//!   { name = "start", type = "point", default = [0.5, 0.0] },
//!   { name = "end", type = "point", default = [0.5, 1.0] },
//!   { name = "jaggedness", type = "float", default = 0.08, min = 0.0, max = 0.5 },
//!   { name = "width", type = "float", default = 0.004, min = 0.0005, max = 0.05 },
//!   { name = "color", type = "color", default = "#a8d8ff" },
//!   { name = "glow", type = "float", default = 1.5, min = 0.0, max = 6.0 },
//!   { name = "flicker", type = "float", default = 20.0, min = 0.0, max = 60.0, desc = "Re-randomize rate (per second)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let a = vec2f(aspect(), 1.0);
    let s = p.start * a;
    let e = p.end * a;
    let q = uv * a;
    let ab = e - s;
    let len = length(ab);
    let dir = ab / max(len, 0.0001);
    let nrm = vec2f(-dir.y, dir.x);
    let t = clamp(dot(q - s, dir) / max(len, 0.0001), 0.0, 1.0);
    let f = floor(time() * p.flicker) + seed() * 17.0;
    let off = fbm(vec2f(t * 8.0, f), 5) * p.jaggedness * sin(t * PI) * len;
    let d = abs(dot(q - s, nrm) - off);
    let core = smoothstep(p.width, 0.0, d);
    let glow = p.width * p.glow * 0.5 / (d + p.width * 0.5) * 0.3;
    let v = core + glow;
    let c = mix(p.color.rgb, vec3f(1.0), core) * v;
    return vec4f(c, clamp(v, 0.0, 1.0));
}
