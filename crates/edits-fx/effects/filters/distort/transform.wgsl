//! id = "transform"
//! name = "Transform"
//! kind = "filter"
//! category = "distort"
//! description = "Translate / scale / rotate the image as an effect (stackable on top of the clip transform; used by zoom punch & shake presets)."
//! tags = ["transform", "zoom", "move", "rotate", "scale"]
//! params = [
//!   { name = "offset", type = "vec2", default = [0.0, 0.0], desc = "Translation in pixels" },
//!   { name = "scale", type = "float", default = 1.0, min = 0.01, max = 20.0 },
//!   { name = "scale_xy", type = "vec2", default = [1.0, 1.0], desc = "Non-uniform scale multiplier" },
//!   { name = "rotation", type = "angle", default = 0.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5], desc = "Pivot (uv)" },
//!   { name = "edge", type = "enum", options = ["transparent", "clamp", "mirror", "wrap"], default = "transparent" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var d = (uv - p.center - p.offset * texel()) * vec2f(aspect(), 1.0);
    d = rot2(-radians(p.rotation)) * d;
    d = d / (max(p.scale, 0.0001) * max(p.scale_xy, vec2f(0.0001)));
    return src_edge(d / vec2f(aspect(), 1.0) + p.center, p.edge);
}
