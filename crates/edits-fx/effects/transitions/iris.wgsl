//! id = "iris"
//! name = "Iris (Circle)"
//! kind = "transition"
//! category = "wipe"
//! description = "Expanding circle reveals the next clip."
//! tags = ["iris", "circle", "reveal"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "softness", type = "float", default = 0.05, min = 0.0, max = 1.0 },
//!   { name = "border", type = "float", default = 0.0, min = 0.0, max = 0.2, desc = "Colored ring width" },
//!   { name = "border_color", type = "color", default = "#ffffff" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = length(centered(uv, p.center));
    let maxr = length(vec2f(max(p.center.x, 1.0 - p.center.x) * aspect(), max(p.center.y, 1.0 - p.center.y)));
    let r = progress() * (maxr + p.softness + p.border);
    let k = smoothstep(r - p.softness - 0.0001, r, d);
    var c = mix(to_img(uv), from_img(uv), k);
    if (p.border > 0.0) {
        let ring = smoothstep(p.border, 0.0, abs(d - r + p.border * 0.5)) * step(0.001, progress()) * step(progress(), 0.999);
        c = mix(c, vec4f(p.border_color.rgb, 1.0), ring);
    }
    return c;
}
