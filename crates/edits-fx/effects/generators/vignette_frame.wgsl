//! id = "film_frame"
//! name = "Film Frame Overlay"
//! kind = "generator"
//! category = "texture"
//! description = "Old film frame: dust, scratches and dark rounded borders (multiply/screen over footage)."
//! tags = ["film", "dust", "scratches", "vintage", "overlay"]
//! params = [
//!   { name = "dust", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "scratches", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "border", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let f = floor(time() * 24.0);
    var v = 1.0;
    let dq = floor(uv * res() / 3.0);
    v -= step(1.0 - p.dust * 0.003, hash12(dq + f * 3.3)) * 0.8;
    let sx = hash11(f * 1.7);
    v -= smoothstep(0.0015, 0.0, abs(uv.x - sx)) * step(0.6, hash11(f * 2.9)) * p.scratches;
    let d = abs(uv - 0.5) * 2.0;
    v *= 1.0 - p.border * smoothstep(0.85, 1.0, max(d.x, d.y) + 0.1 * length(d));
    return vec4f(vec3f(max(v, 0.0)), 1.0);
}
