//! id = "turbulent_displace"
//! name = "Turbulent Displace"
//! kind = "filter"
//! category = "distort"
//! description = "Fractal-noise displacement: liquid, smoke, melting, dreamy wobble."
//! tags = ["displace", "noise", "liquid", "turbulence", "melt"]
//! params = [
//!   { name = "amount", type = "float", default = 20.0, min = 0.0, max = 400.0, desc = "Pixels" },
//!   { name = "size", type = "float", default = 200.0, min = 4.0, max = 3000.0, desc = "Noise scale in pixels" },
//!   { name = "speed", type = "float", default = 0.5, min = 0.0, max = 10.0 },
//!   { name = "octaves", type = "int", default = 4, min = 1, max = 8 },
//!   { name = "edge", type = "enum", options = ["transparent", "clamp", "mirror", "wrap"], default = "clamp" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * res() / p.size;
    let t = time() * p.speed + seed() * 10.0;
    let o = i32(clamp(p.octaves, 1.0, 8.0));
    let d = vec2f(fbm(q + vec2f(t, 0.0), o), fbm(q + vec2f(5.2, 1.3 - t), o));
    return src_edge(uv + d * p.amount * texel(), p.edge);
}
