//! id = "halftone"
//! name = "Halftone"
//! kind = "filter"
//! category = "stylize"
//! description = "Print halftone dots (mono or CMY), rotated grid."
//! tags = ["halftone", "comic", "print", "dots", "pop art"]
//! params = [
//!   { name = "size", type = "float", default = 8.0, min = 2.0, max = 64.0, desc = "Dot spacing in pixels" },
//!   { name = "angle", type = "angle", default = 45.0 },
//!   { name = "color_mode", type = "enum", options = ["mono", "color", "source_tint"], default = "source_tint" },
//!   { name = "ink", type = "color", default = "#000000" },
//!   { name = "paper", type = "color", default = "#ffffff" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn dot_at(px: vec2f, ang: f32, s: f32, v: f32) -> f32 {
    let r = rot2(ang) * px;
    let cell = fract(r / s) - 0.5;
    let rad = sqrt(clamp(v, 0.0, 1.0)) * 0.7;
    return 1.0 - smoothstep(rad - 0.08, rad + 0.08, length(cell));
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c0 = src_clamp(uv);
    let c = grade_begin(c0);
    let px = uv * res();
    let a = radians(p.angle);
    var o = vec3f(0.0);
    let m = i32(p.color_mode);
    if (m == 1) {
        let cmy = 1.0 - c.rgb;
        let dc = dot_at(px, a + 0.26, p.size, cmy.r);
        let dm = dot_at(px, a + 1.31, p.size, cmy.g);
        let dy = dot_at(px, a, p.size, cmy.b);
        o = 1.0 - vec3f(dc, dm, dy);
    } else {
        let d = dot_at(px, a, p.size, 1.0 - luma(c.rgb));
        let paper = to_srgb(p.paper.rgb);
        var ink = to_srgb(p.ink.rgb);
        if (m == 2) { ink = c.rgb * 0.5; }
        o = mix(paper, ink, d);
    }
    return grade_end(vec4f(mix(c.rgb, o, p.amount), c.a));
}
