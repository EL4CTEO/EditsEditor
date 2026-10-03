//! id = "additive_dissolve"
//! name = "Additive Dissolve"
//! kind = "transition"
//! category = "basic"
//! description = "Bright, glowing dissolve where both images add up in the middle."
//! tags = ["dissolve", "light", "glow", "dreamy"]
//! params = [ { name = "boost", type = "float", default = 1.0, min = 0.0, max = 3.0 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let a = from_img(uv) * min(1.0, (1.0 - t) * 2.0);
    let b = to_img(uv) * min(1.0, t * 2.0);
    let peak = sin(t * PI) * p.boost * 0.3;
    let c = a + b;
    return vec4f(c.rgb * (1.0 + peak), min(c.a, 1.0));
}
