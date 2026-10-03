//! id = "rounded_corners"
//! name = "Rounded Corners / Border"
//! kind = "filter"
//! category = "stylize"
//! description = "Round the frame corners with an optional colored border (picture-in-picture cards)."
//! tags = ["rounded", "corners", "border", "frame", "card"]
//! params = [
//!   { name = "radius", type = "float", default = 40.0, min = 0.0, max = 1000.0, desc = "Pixels" },
//!   { name = "border", type = "float", default = 0.0, min = 0.0, max = 100.0, desc = "Border width (pixels)" },
//!   { name = "border_color", type = "color", default = "#ffffff" },
//!   { name = "inset", type = "float", default = 0.0, min = 0.0, max = 500.0, desc = "Shrink the card (pixels)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let px = (uv - 0.5) * res();
    let half_size = res() * 0.5 - p.inset;
    let d = sd_box(px, half_size - p.radius) - p.radius;
    let c = src_clamp(uv);
    let a = 1.0 - smoothstep(-0.75, 0.75, d);
    let bw = smoothstep(-p.border - 0.75, -p.border + 0.75, d);
    let col = mix(c, vec4f(p.border_color.rgb, 1.0), bw * select(0.0, 1.0, p.border > 0.0));
    return col * a;
}
