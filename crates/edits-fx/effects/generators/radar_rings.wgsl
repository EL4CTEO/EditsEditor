//! id = "concentric_rings"
//! name = "Concentric Rings"
//! kind = "generator"
//! category = "background"
//! description = "Animated concentric circles (hypnotic / tunnel / radar backgrounds)."
//! tags = ["rings", "circles", "hypnotic", "tunnel", "background"]
//! params = [
//!   { name = "frequency", type = "float", default = 20.0, min = 1.0, max = 200.0 },
//!   { name = "speed", type = "float", default = 1.0, min = -20.0, max = 20.0 },
//!   { name = "color_a", type = "color", default = "#000000" },
//!   { name = "color_b", type = "color", default = "#ffffff" },
//!   { name = "sharpness", type = "float", default = 0.8, min = 0.0, max = 1.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let r = length(centered(uv, p.center));
    let s = sin(r * p.frequency - time() * p.speed * TAU) * 0.5 + 0.5;
    let k = smoothstep(0.5 - (1.0 - p.sharpness) * 0.5 - 0.01, 0.5 + (1.0 - p.sharpness) * 0.5 + 0.01, s);
    return vec4f(mix(p.color_a.rgb, p.color_b.rgb, k), 1.0);
}
