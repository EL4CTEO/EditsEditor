//! id = "wipe"
//! name = "Linear Wipe"
//! kind = "transition"
//! category = "wipe"
//! description = "Straight edge wipe at any angle with soft edge."
//! tags = ["wipe", "linear"]
//! params = [
//!   { name = "angle", type = "angle", default = 0.0, desc = "Direction of travel (0 = left to right)" },
//!   { name = "softness", type = "float", default = 0.05, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle);
    let a = vec2f(aspect(), 1.0);
    let ext = (abs(d.x) * aspect() + abs(d.y)) * 0.5;
    let x = dot((uv - 0.5) * a, d) / ext * 0.5 + 0.5;
    let s = p.softness;
    let k = smoothstep(progress() * (1.0 + s) - s, progress() * (1.0 + s), 1.0 - x);
    return mix(from_img(uv), to_img(uv), 1.0 - k);
}
