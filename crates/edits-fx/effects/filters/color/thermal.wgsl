//! id = "thermal"
//! name = "Thermal Vision"
//! kind = "filter"
//! category = "color"
//! description = "Heat-camera false color."
//! tags = ["thermal", "heat", "false color", "predator"]
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = luma(c.rgb);
    let h = vec3f(clamp(l * 3.0, 0.0, 1.0), clamp(l * 3.0 - 1.0, 0.0, 1.0), clamp(l * 3.0 - 2.0, 0.0, 1.0));
    let t = mix(vec3f(0.05, 0.0, 0.35), vec3f(1.0, 0.25, 0.1), h.x) + vec3f(0.0, h.y * 0.8, 0.0) + vec3f(h.z);
    return grade_end(vec4f(mix(c.rgb, t, p.amount), c.a));
}
