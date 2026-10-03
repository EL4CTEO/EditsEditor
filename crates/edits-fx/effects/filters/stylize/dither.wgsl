//! id = "dither"
//! name = "Dither"
//! kind = "filter"
//! category = "stylize"
//! description = "Ordered Bayer dithering with retro palettes (1-bit, Game Boy, CGA, color)."
//! tags = ["dither", "retro", "pixel", "1bit", "gameboy", "lofi"]
//! params = [
//!   { name = "palette", type = "enum", options = ["mono", "gameboy", "cga", "color", "sepia"], default = "gameboy" },
//!   { name = "levels", type = "float", default = 4.0, min = 2.0, max = 16.0, desc = "Levels per channel (color palette)" },
//!   { name = "pixel_size", type = "float", default = 2.0, min = 1.0, max = 16.0 },
//! ]
fn bayer(p: vec2i) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    let i = (p.x & 3) + (p.y & 3) * 4;
    return m[i] / 16.0 - 0.5;
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let px = floor(uv * res() / p.pixel_size);
    let c = grade_begin(src_clamp((px + 0.5) * p.pixel_size / res()));
    let b = bayer(vec2i(px));
    let m = i32(p.palette);
    var o = vec3f(0.0);
    if (m == 3) {
        let n = p.levels - 1.0;
        o = floor(c.rgb * n + 0.5 + b) / n;
    } else {
        let l = clamp(luma(c.rgb) + b * 0.25, 0.0, 0.999);
        if (m == 0) { o = vec3f(step(0.5, l)); }
        else if (m == 1) {
            var pal = array<vec3f, 4>(vec3f(0.06, 0.22, 0.06), vec3f(0.19, 0.38, 0.19), vec3f(0.55, 0.67, 0.06), vec3f(0.61, 0.74, 0.06));
            o = pal[i32(l * 4.0)];
        } else if (m == 2) {
            var pal = array<vec3f, 4>(vec3f(0.0), vec3f(0.33, 1.0, 1.0), vec3f(1.0, 0.33, 1.0), vec3f(1.0));
            o = pal[i32(l * 4.0)];
        } else {
            o = mix(vec3f(0.2, 0.12, 0.06), vec3f(1.0, 0.92, 0.75), floor(l * 4.0) / 3.0);
        }
    }
    return grade_end(vec4f(o, c.a));
}
