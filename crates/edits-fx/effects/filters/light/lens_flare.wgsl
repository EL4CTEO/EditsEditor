//! id = "lens_flare"
//! name = "Lens Flare"
//! kind = "filter"
//! category = "light"
//! description = "Procedural lens flare with glare, halo ring and ghosts, positioned at a light point."
//! tags = ["flare", "lens", "light", "sun"]
//! params = [
//!   { name = "position", type = "point", default = [0.75, 0.25], desc = "Light position" },
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 5.0 },
//!   { name = "color", type = "color", default = "#ffd9a0" },
//!   { name = "ghosts", type = "float", default = 1.0, min = 0.0, max = 3.0, desc = "Ghost brightness" },
//!   { name = "halo", type = "float", default = 0.6, min = 0.0, max = 3.0 },
//! ]
fn flare(uv: vec2f, pos: vec2f) -> vec3f {
    let a = vec2f(aspect(), 1.0);
    let d = (uv - pos) * a;
    let r = length(d);
    var c = vec3f(0.0);
    c += vec3f(1.0) * 0.03 / (r * r + 0.003);
    let ang = atan2(d.y, d.x);
    c += vec3f(1.0) * pow(max(0.0, sin(ang * 6.0 + 1.0)), 30.0) * 0.08 / (r + 0.08);
    return c;
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let a = vec2f(aspect(), 1.0);
    var c = flare(uv, p.position) * p.color.rgb;
    let axis = vec2f(0.5) - p.position;
    for (var i = 1; i <= 5; i++) {
        let f = f32(i) * 0.38;
        let gp = p.position + axis * f * 2.0;
        let gr = 0.02 + 0.03 * hash11(f32(i));
        let dd = length((uv - gp) * a);
        let tint = hsv2rgb(vec3f(fract(f32(i) * 0.17 + 0.05), 0.6, 1.0));
        c += tint * smoothstep(gr, gr * 0.6, dd) * 0.25 * p.ghosts;
    }
    let hr = length((uv - vec2f(0.5) - axis * 0.3) * a);
    let ring = smoothstep(0.03, 0.0, abs(hr - 0.35)) * 0.15 * p.halo;
    c += hsv2rgb(vec3f(fract(hr * 2.0), 0.5, 1.0)) * ring;
    c *= p.intensity;
    let o = src(uv);
    return vec4f(screen3(o.rgb, c), max(o.a, clamp(luma(c), 0.0, 1.0)));
}
