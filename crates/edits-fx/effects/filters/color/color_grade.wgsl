//! id = "color_grade"
//! name = "Color Grade Looks"
//! kind = "filter"
//! category = "color"
//! description = "One-click cinematic looks: teal_orange, anime_vivid, cyberpunk, sunset, vintage, bleach_bypass, noir, cold, warm, pastel, matrix, dreamy, horror, golden_hour."
//! tags = ["color", "grade", "look", "cinematic", "anime", "cyberpunk"]
//! params = [
//!   { name = "look", type = "enum", options = ["teal_orange", "anime_vivid", "cyberpunk", "sunset", "vintage", "bleach_bypass", "noir", "cold", "warm", "pastel", "matrix", "dreamy", "horror", "golden_hour"], default = "teal_orange" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.5 },
//! ]
fn sat(c: vec3f, s: f32) -> vec3f { return mix(vec3f(luma(c)), c, s); }
fn contrast(c: vec3f, k: f32) -> vec3f { return (c - 0.5) * k + 0.5; }
fn split(c: vec3f, sh: vec3f, hi: vec3f) -> vec3f {
    let l = luma(c);
    return c + sh * (1.0 - smoothstep(0.0, 0.55, l)) + hi * smoothstep(0.45, 1.0, l);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let x = c.rgb;
    var o = x;
    let m = i32(p.look);
    switch m {
        case 0: { o = contrast(split(x, vec3f(-0.05, 0.03, 0.08), vec3f(0.08, 0.02, -0.06)), 1.1); }
        case 1: { o = contrast(sat(x, 1.45), 1.08); o = split(o, vec3f(0.0, 0.0, 0.04), vec3f(0.03, 0.01, -0.02)); }
        case 2: { o = contrast(split(sat(x, 1.3), vec3f(0.06, -0.04, 0.12), vec3f(0.1, -0.02, 0.08)), 1.15); }
        case 3: { o = split(sat(x, 1.2), vec3f(0.04, -0.02, 0.06), vec3f(0.12, 0.04, -0.08)); }
        case 4: { o = contrast(sat(x, 0.7), 0.9); o = split(o, vec3f(0.06, 0.04, 0.0), vec3f(0.05, 0.03, -0.04)) + 0.03; }
        case 5: { let l = vec3f(luma(x)); o = contrast(mix(x, l, 0.55), 1.3); }
        case 6: { let l = luma(x); o = contrast(vec3f(l), 1.45); }
        case 7: { o = split(sat(x, 0.85), vec3f(-0.02, 0.02, 0.07), vec3f(-0.03, 0.02, 0.06)); }
        case 8: { o = split(sat(x, 1.1), vec3f(0.04, 0.01, -0.03), vec3f(0.07, 0.03, -0.05)); }
        case 9: { o = mix(sat(x, 0.75), vec3f(1.0), 0.15); o = split(o, vec3f(0.03, 0.0, 0.05), vec3f(0.04, 0.0, 0.03)); }
        case 10: { o = vec3f(x.r * 0.6, x.g * 1.1 + 0.03, x.b * 0.6); o = contrast(o, 1.15); }
        case 11: { o = mix(sat(x, 1.15), screen3(x, vec3f(0.12, 0.06, 0.15)), 0.6); }
        case 12: { o = contrast(split(sat(x, 0.55), vec3f(0.0, 0.03, 0.02), vec3f(0.02, 0.04, -0.02)), 1.3) - 0.04; }
        default: { o = split(sat(x, 1.15), vec3f(0.02, 0.0, -0.02), vec3f(0.14, 0.07, -0.05)); }
    }
    return grade_end(vec4f(mix(x, o, p.amount), c.a));
}
