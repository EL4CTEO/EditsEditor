//! id = "film_grain"
//! name = "Film Grain"
//! kind = "filter"
//! category = "color"
//! description = "Animated photographic grain, stronger in midtones."
//! tags = ["grain", "film", "noise", "texture", "cinematic"]
//! params = [
//!   { name = "amount", type = "float", default = 0.12, min = 0.0, max = 1.0 },
//!   { name = "size", type = "float", default = 1.5, min = 0.5, max = 8.0 },
//!   { name = "color", type = "bool", default = false, desc = "Colored grain" },
//!   { name = "fps", type = "float", default = 24.0, min = 1.0, max = 60.0, desc = "Grain refresh rate" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let f = floor(time() * p.fps);
    let q = floor(uv * res() / p.size);
    let n = hash12(q + f * 17.31) - 0.5;
    var g = vec3f(n);
    if (p.color > 0.5) {
        g = vec3f(n, hash12(q + f * 9.1 + 3.0) - 0.5, hash12(q + f * 5.7 + 7.0) - 0.5);
    }
    let l = luma(c.rgb);
    let w = 1.0 - pow(abs(l - 0.5) * 2.0, 2.0);
    return vec4f(max(c.rgb + g * p.amount * (0.35 + w) * c.a, vec3f(0.0)), c.a);
}
