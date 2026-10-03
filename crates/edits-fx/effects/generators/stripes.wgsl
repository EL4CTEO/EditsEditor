//! id = "stripes"
//! name = "Animated Stripes"
//! kind = "generator"
//! category = "background"
//! description = "Diagonal moving stripes (warning tape, energetic title backgrounds)."
//! tags = ["stripes", "diagonal", "background", "energy"]
//! params = [
//!   { name = "width", type = "float", default = 40.0, min = 2.0, max = 500.0, desc = "Stripe width in pixels" },
//!   { name = "angle", type = "angle", default = 45.0 },
//!   { name = "speed", type = "float", default = 60.0, min = -2000.0, max = 2000.0, desc = "Pixels per second" },
//!   { name = "color_a", type = "color", default = "#ffde00" },
//!   { name = "color_b", type = "color", default = "#111111" },
//!   { name = "ratio", type = "float", default = 0.5, min = 0.05, max = 0.95 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let x = dot((uv - 0.5) * res(), dir_deg(p.angle)) - time() * p.speed;
    let f = fract(x / (p.width * 2.0));
    let k = smoothstep(p.ratio - 0.01, p.ratio + 0.01, f);
    return vec4f(mix(p.color_a.rgb, p.color_b.rgb, k), 1.0);
}
