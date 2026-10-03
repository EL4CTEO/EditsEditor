//! id = "doors"
//! name = "Doors"
//! kind = "transition"
//! category = "geometric"
//! description = "Outgoing image splits in two halves that slide apart."
//! tags = ["doors", "split", "open", "reveal"]
//! params = [
//!   { name = "vertical", type = "bool", default = false, desc = "Split top/bottom instead of left/right" },
//!   { name = "gap_color", type = "color", default = "#000000" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let e = t * t * (3.0 - 2.0 * t);
    let s = e * 0.5;
    var c = vec4f(0.0);
    if (p.vertical > 0.5) {
        if (uv.y < 0.5) { c = src(uv + vec2f(0.0, s)) * select(0.0, 1.0, uv.y + s < 0.5); }
        else { c = src(uv - vec2f(0.0, s)) * select(0.0, 1.0, uv.y - s > 0.5); }
    } else {
        if (uv.x < 0.5) { c = src(uv + vec2f(s, 0.0)) * select(0.0, 1.0, uv.x + s < 0.5); }
        else { c = src(uv - vec2f(s, 0.0)) * select(0.0, 1.0, uv.x - s > 0.5); }
    }
    return over(c, to_img(uv));
}
