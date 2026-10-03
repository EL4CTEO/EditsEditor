//! id = "aurora"
//! name = "Aurora"
//! kind = "generator"
//! category = "space"
//! description = "Flowing aurora borealis curtains."
//! tags = ["aurora", "northern lights", "sky", "night", "dreamy"]
//! params = [
//!   { name = "speed", type = "float", default = 0.3, min = 0.0, max = 5.0 },
//!   { name = "color_a", type = "color", default = "#27ff9a" },
//!   { name = "color_b", type = "color", default = "#7b2bff" },
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 4.0 },
//!   { name = "background", type = "color", default = "#02030a" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var c = p.background.rgb;
    let t = time() * p.speed;
    for (var i = 0; i < 5; i++) {
        let fi = f32(i);
        let y = 0.25 + fi * 0.07 + fbm(vec2f(uv.x * 2.0 + t * 0.3 + fi, t * 0.2), 4) * 0.15;
        let d = uv.y - y;
        let band = exp(-d * d * 300.0) + smoothstep(0.0, -0.25, d) * smoothstep(-0.4, 0.0, d) * 0.15 * (1.0 + sin(uv.x * 40.0 + t * 3.0 + fi));
        c += mix(p.color_a.rgb, p.color_b.rgb, fi / 4.0) * band * p.intensity * 0.5;
    }
    return vec4f(c, 1.0);
}
