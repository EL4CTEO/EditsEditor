//! id = "film_burn"
//! name = "Film Burn"
//! kind = "transition"
//! category = "organic"
//! description = "Hot orange film burn blooms over the cut."
//! tags = ["burn", "film", "light leak", "warm", "vintage"]
//! params = [
//!   { name = "color", type = "color", default = "#ff7a1a" },
//!   { name = "intensity", type = "float", default = 1.5, min = 0.0, max = 5.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let q = uv * vec2f(aspect(), 1.0) * 2.5;
    let n = fbm(q + vec2f(t * 2.0, seed() * 5.0), 5) * 0.5 + 0.5;
    let bloom = smoothstep(0.0, 1.0, sin(t * PI) * 1.5 * n);
    let base = mix(from_img(uv), to_img(uv), smoothstep(0.35, 0.65, t));
    let hot = p.color.rgb * bloom * p.intensity + vec3f(1.0, 0.9, 0.7) * pow(bloom, 3.0) * p.intensity;
    return vec4f(screen3(base.rgb, hot), max(base.a, bloom));
}
