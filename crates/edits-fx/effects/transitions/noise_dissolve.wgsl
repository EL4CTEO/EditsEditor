//! id = "noise_dissolve"
//! name = "Noise Dissolve"
//! kind = "transition"
//! category = "organic"
//! description = "Organic noise threshold dissolve with a glowing burning edge."
//! tags = ["dissolve", "noise", "burn", "organic", "magic"]
//! params = [
//!   { name = "scale", type = "float", default = 6.0, min = 0.5, max = 50.0 },
//!   { name = "edge_width", type = "float", default = 0.05, min = 0.0, max = 0.3 },
//!   { name = "edge_color", type = "color", default = "#ff8a00" },
//!   { name = "edge_glow", type = "float", default = 3.0, min = 0.0, max = 10.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let n = fbm(uv * vec2f(aspect(), 1.0) * p.scale + seed() * 7.0, 5) * 0.5 + 0.5;
    let t = progress() * (1.0 + p.edge_width * 2.0) - p.edge_width;
    let k = smoothstep(t - 0.01, t + 0.01, n);
    let edge = (1.0 - smoothstep(0.0, p.edge_width + 0.0001, abs(n - t))) * step(0.001, progress()) * step(progress(), 0.999);
    let c = mix(to_img(uv), from_img(uv), k);
    return vec4f(c.rgb + p.edge_color.rgb * edge * p.edge_glow, max(c.a, edge));
}
