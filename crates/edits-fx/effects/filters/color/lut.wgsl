//! id = "lut"
//! name = "LUT (3D .cube)"
//! kind = "filter"
//! category = "color"
//! description = "Apply a 3D color lookup table from a .cube asset."
//! tags = ["color", "lut", "grade", "cube"]
//! params = [
//!   { name = "lut", type = "lut", default = "", desc = "Asset id of an imported .cube file" },
//!   { name = "size", type = "float", default = 33.0, desc = "Set automatically from the LUT" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn lut_sample(c: vec3f, n: f32) -> vec3f {
    let cc = saturate3(c) * (n - 1.0);
    let b0 = floor(cc.b);
    let b1 = min(b0 + 1.0, n - 1.0);
    let fb = cc.b - b0;
    let w = n * n;
    let y = (cc.g + 0.5) / n;
    let x0 = (b0 * n + cc.r + 0.5) / w;
    let x1 = (b1 * n + cc.r + 0.5) / w;
    let a = textureSampleLevel(t2, samp, vec2f(x0, y), 0.0).rgb;
    let b = textureSampleLevel(t2, samp, vec2f(x1, y), 0.0).rgb;
    return mix(a, b, fb);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    if (textureDimensions(t2).x < 4u) { return src_clamp(uv); }
    let c = grade_begin(src_clamp(uv));
    let l = lut_sample(c.rgb, max(p.size, 2.0));
    return grade_end(vec4f(mix(c.rgb, l, p.amount), c.a));
}
