//! id = "color_phase"
//! name = "Color Phase"
//! kind = "transition"
//! category = "light"
//! description = "Each color channel transitions at a different moment."
//! tags = ["color", "phase", "rgb", "psychedelic"]
//! params = [ { name = "spread", type = "float", default = 0.4, min = 0.0, max = 0.9 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let a = from_img(uv);
    let b = to_img(uv);
    let s = p.spread;
    let k = clamp((vec3f(t) - vec3f(0.0, s * 0.5, s)) / (1.0 - s), vec3f(0.0), vec3f(1.0));
    return vec4f(mix(a.rgb, b.rgb, k), mix(a.a, b.a, t));
}
