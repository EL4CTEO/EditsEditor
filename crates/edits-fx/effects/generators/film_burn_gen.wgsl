//! id = "film_burn_overlay"
//! name = "Film Burn Overlay"
//! kind = "generator"
//! category = "light"
//! description = "Burning film edges with hot cores, on black (use screen/add blend)."
//! tags = ["film burn", "overlay", "vintage", "fire"]
//! params = [
//!   { name = "speed", type = "float", default = 0.5, min = 0.0, max = 5.0 },
//!   { name = "amount", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "color", type = "color", default = "#ff6a10" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = time() * p.speed;
    let n = fbm(uv * vec2f(aspect(), 1.0) * 2.5 + vec2f(t, seed() * 9.0), 5) * 0.5 + 0.5;
    let edge = max(max(1.0 - uv.x, uv.x), max(1.0 - uv.y, uv.y));
    let v = smoothstep(1.0 - p.amount, 1.0, n * 0.6 + edge * 0.6);
    let c = p.color.rgb * v * 2.0 + vec3f(1.0, 0.9, 0.6) * pow(v, 4.0) * 2.0;
    return vec4f(c, 1.0);
}
