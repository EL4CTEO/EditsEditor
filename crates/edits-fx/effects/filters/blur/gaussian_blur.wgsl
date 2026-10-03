//! id = "gaussian_blur"
//! name = "Gaussian Blur"
//! kind = "filter"
//! category = "blur"
//! description = "High quality separable gaussian blur."
//! tags = ["blur", "soft", "defocus"]
//! params = [
//!   { name = "radius", type = "float", default = 12.0, min = 0.0, max = 200.0, desc = "Blur radius in pixels" },
//!   { name = "dimensions", type = "enum", options = ["both", "horizontal", "vertical"], default = "both", desc = "Blur direction" },
//! ]
//! passes = [ { entry = "blur_h" }, { entry = "blur_v" } ]
fn blur_h(uv: vec2f, p: Params) -> vec4f {
    let r = select(p.radius, 0.0, i32(p.dimensions) == 2);
    return blur_dir(uv, vec2f(1.0, 0.0), r);
}
fn blur_v(uv: vec2f, p: Params) -> vec4f {
    let r = select(p.radius, 0.0, i32(p.dimensions) == 1);
    return blur_dir(uv, vec2f(0.0, 1.0), r);
}
