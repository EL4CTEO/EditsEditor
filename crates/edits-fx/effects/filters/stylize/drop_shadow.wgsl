//! id = "drop_shadow"
//! name = "Drop Shadow"
//! kind = "filter"
//! category = "stylize"
//! description = "Soft shadow behind the layer's alpha."
//! tags = ["shadow", "drop shadow", "depth"]
//! params = [
//!   { name = "offset", type = "vec2", default = [8.0, 8.0], desc = "Pixels" },
//!   { name = "blur", type = "float", default = 12.0, min = 0.0, max = 100.0 },
//!   { name = "color", type = "color", default = "#000000" },
//!   { name = "opacity", type = "float", default = 0.6, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src(uv);
    let base = uv - p.offset * texel();
    var a = 0.0;
    let golden = 2.39996323;
    let n = 48;
    for (var i = 0; i < n; i++) {
        let r = sqrt((f32(i) + 0.5) / f32(n)) * p.blur;
        let ang = f32(i) * golden;
        a += src(base + vec2f(cos(ang), sin(ang)) * r * texel()).a;
    }
    a = a / f32(n) * p.opacity;
    return over(c, vec4f(p.color.rgb, 1.0) * a);
}
