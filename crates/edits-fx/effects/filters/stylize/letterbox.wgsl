//! id = "letterbox"
//! name = "Letterbox / Cinema Bars"
//! kind = "filter"
//! category = "stylize"
//! description = "Black bars for a cinematic aspect ratio. Animate `amount` to slide them in."
//! tags = ["letterbox", "cinema", "bars", "widescreen", "cinematic"]
//! params = [
//!   { name = "ratio", type = "float", default = 2.39, min = 0.5, max = 4.0, desc = "Target aspect ratio" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "color", type = "color", default = "#000000" },
//!   { name = "vertical", type = "bool", default = false, desc = "Pillarbox (side bars) instead" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    var bar = 0.0;
    if (p.vertical > 0.5) {
        let w = (1.0 - p.ratio / aspect()) * 0.5 * p.amount;
        bar = select(0.0, 1.0, uv.x < w || uv.x > 1.0 - w);
    } else {
        let h = (1.0 - aspect() / p.ratio) * 0.5 * p.amount;
        bar = select(0.0, 1.0, uv.y < h || uv.y > 1.0 - h);
    }
    return mix(c, vec4f(p.color.rgb, 1.0), bar);
}
