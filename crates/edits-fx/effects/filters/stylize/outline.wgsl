//! id = "outline"
//! name = "Outline / Stroke"
//! kind = "filter"
//! category = "stylize"
//! description = "Stroke around the layer's alpha (cut-out characters, text, stickers)."
//! tags = ["outline", "stroke", "border", "sticker", "cutout"]
//! params = [
//!   { name = "width", type = "float", default = 6.0, min = 0.0, max = 60.0, desc = "Pixels" },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "softness", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "only_outline", type = "bool", default = false },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src(uv);
    var a = 0.0;
    let rings = 4;
    let dirs = 16;
    for (var r = 1; r <= rings; r++) {
        let rad = p.width * f32(r) / f32(rings);
        for (var i = 0; i < dirs; i++) {
            let ang = f32(i) / f32(dirs) * TAU + f32(r) * 0.4;
            a = max(a, src(uv + vec2f(cos(ang), sin(ang)) * rad * texel()).a);
        }
    }
    a = smoothstep(0.5 - p.softness * 0.5, 0.5 + p.softness * 0.5 + 0.0001, a);
    let stroke = vec4f(p.color.rgb, 1.0) * a * p.color.a;
    if (p.only_outline > 0.5) { return stroke * (1.0 - c.a); }
    return over(c, stroke);
}
