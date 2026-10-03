//! id = "curves"
//! name = "Curves"
//! kind = "filter"
//! category = "color"
//! description = "Smooth tone curve: control shadows, midtones and highlights for master and each channel. Values are offsets (-1..1) at 25%/50%/75%."
//! tags = ["color", "curves", "contrast", "grade"]
//! params = [
//!   { name = "master", type = "vec3", default = [0.0, 0.0, 0.0], desc = "Offsets at [shadows, mids, highlights]" },
//!   { name = "red", type = "vec3", default = [0.0, 0.0, 0.0] },
//!   { name = "green", type = "vec3", default = [0.0, 0.0, 0.0] },
//!   { name = "blue", type = "vec3", default = [0.0, 0.0, 0.0] },
//! ]
fn curve(x: f32, o: vec3f) -> f32 {
    // smooth bumps centered at .25/.5/.75, zero at the ends
    let s = sin(clamp(x, 0.0, 1.0) * PI);
    let w1 = exp(-pow((x - 0.25) / 0.18, 2.0));
    let w2 = exp(-pow((x - 0.5) / 0.2, 2.0));
    let w3 = exp(-pow((x - 0.75) / 0.18, 2.0));
    return x + s * (o.x * w1 + o.y * w2 + o.z * w3);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var r = curve(c.r, p.master);
    var g = curve(c.g, p.master);
    var b = curve(c.b, p.master);
    r = curve(r, p.red);
    g = curve(g, p.green);
    b = curve(b, p.blue);
    return grade_end(vec4f(r, g, b, c.a));
}
