//! id = "bounce"
//! name = "Bounce Drop"
//! kind = "transition"
//! category = "motion"
//! description = "The next clip drops in from the top and bounces to rest."
//! tags = ["bounce", "drop", "playful"]
//! params = [ { name = "shadow", type = "float", default = 0.5, min = 0.0, max = 1.0 } ]
fn bounce_out(t: f32) -> f32 {
    if (t < 1.0 / 2.75) { return 7.5625 * t * t; }
    if (t < 2.0 / 2.75) { let x = t - 1.5 / 2.75; return 7.5625 * x * x + 0.75; }
    if (t < 2.5 / 2.75) { let x = t - 2.25 / 2.75; return 7.5625 * x * x + 0.9375; }
    let x = t - 2.625 / 2.75;
    return 7.5625 * x * x + 0.984375;
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let y = 1.0 - bounce_out(progress());
    let b = orig(uv + vec2f(0.0, y));
    let shadow = smoothstep(0.0, 0.05, 1.0 - y - uv.y) * 0.0 + (1.0 - smoothstep(0.0, 0.08, abs(uv.y - (1.0 - y)))) * p.shadow * select(0.0, 1.0, uv.y > 1.0 - y);
    let a = from_img(uv);
    return over(b, vec4f(a.rgb * (1.0 - shadow), a.a));
}
