//! id = "polka_curtain"
//! name = "Polka Dot Curtain"
//! kind = "transition"
//! category = "geometric"
//! description = "Growing dots in a grid reveal the next clip."
//! tags = ["dots", "polka", "circles", "pop", "cute"]
//! params = [
//!   { name = "dots", type = "float", default = 12.0, min = 2.0, max = 60.0 },
//!   { name = "stagger", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let g = vec2f(p.dots * aspect(), p.dots);
    let cell = floor(uv * g);
    let f = fract(uv * g) - 0.5;
    let delay = (cell.x / g.x) * p.stagger;
    let t = clamp((progress() - delay) / max(1.0 - p.stagger, 0.0001), 0.0, 1.0);
    let k = step(length(f), t * 0.75);
    return mix(from_img(uv), to_img(uv), k);
}
