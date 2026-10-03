//! id = "halftone_pattern"
//! name = "Halftone Pattern"
//! kind = "generator"
//! category = "anime"
//! description = "Manga-style halftone dot gradient background (comic panels, title cards)."
//! tags = ["halftone", "manga", "comic", "dots", "background"]
//! params = [
//!   { name = "size", type = "float", default = 18.0, min = 3.0, max = 200.0 },
//!   { name = "angle", type = "angle", default = 45.0, desc = "Gradient direction" },
//!   { name = "dot_color", type = "color", default = "#000000" },
//!   { name = "background", type = "color", default = "#ffffff" },
//!   { name = "radial", type = "bool", default = false },
//!   { name = "scroll", type = "float", default = 0.0, min = -500.0, max = 500.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var g = dot(uv - 0.5, dir_deg(p.angle)) + 0.5;
    if (p.radial > 0.5) { g = 1.0 - length(centered(uv, vec2f(0.5))) * 1.3; }
    let px = rot2(0.785) * (uv * res()) + vec2f(time() * p.scroll, 0.0);
    let f = fract(px / p.size) - 0.5;
    let r = sqrt(clamp(g, 0.0, 1.0)) * 0.7;
    let k = 1.0 - smoothstep(r - 0.04, r + 0.04, length(f));
    return vec4f(mix(p.background.rgb, p.dot_color.rgb, k), 1.0);
}
