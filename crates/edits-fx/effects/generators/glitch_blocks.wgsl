//! id = "glitch_blocks"
//! name = "Glitch Blocks"
//! kind = "generator"
//! category = "texture"
//! description = "Random flashing digital blocks and bars (overlay with add/screen/difference)."
//! tags = ["glitch", "blocks", "digital", "overlay", "cyber"]
//! params = [
//!   { name = "density", type = "float", default = 0.15, min = 0.0, max = 1.0 },
//!   { name = "speed", type = "float", default = 12.0, min = 0.5, max = 60.0 },
//!   { name = "block", type = "float", default = 40.0, min = 4.0, max = 400.0 },
//!   { name = "colored", type = "bool", default = true },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = floor(time() * p.speed) + seed() * 31.0;
    let b = floor(uv * res() / (p.block * vec2f(hash11(t) * 4.0 + 1.0, 1.0)));
    let h = hash12(b + t);
    let on = step(1.0 - p.density, h);
    var c = vec3f(1.0);
    if (p.colored > 0.5) { c = to_linear(hsv2rgb(vec3f(hash12(b * 1.3 + t), 0.9, 1.0))); }
    return vec4f(c * on, on);
}
