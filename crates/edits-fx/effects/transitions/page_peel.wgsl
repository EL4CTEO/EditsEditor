//! id = "page_peel"
//! name = "Page Peel"
//! kind = "transition"
//! category = "geometric"
//! description = "Page curls from a corner revealing the next clip (manga page turn)."
//! tags = ["page", "curl", "manga", "book"]
//! params = [ { name = "shadow", type = "float", default = 0.5, min = 0.0, max = 1.0 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let dir = normalize(vec2f(-1.0, -0.6));
    let pos = mix(vec2f(1.1, 1.1), vec2f(-0.6, -0.6), t);
    let d = dot(uv - pos, dir);
    if (d > 0.0) {
        return to_img(uv) * (1.0 - p.shadow * exp(-d * 30.0));
    }
    let r = 0.08;
    let folded = uv - dir * (2.0 * d);
    if (-d < r * PI && all(folded >= vec2f(0.0)) && all(folded <= vec2f(1.0))) {
        let back = from_img(folded);
        let shade = 0.7 + 0.3 * cos(-d / r);
        return vec4f(back.rgb * shade, back.a);
    }
    return from_img(uv);
}
