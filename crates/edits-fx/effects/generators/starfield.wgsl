//! id = "starfield"
//! name = "Starfield / Warp"
//! kind = "generator"
//! category = "space"
//! description = "Stars flying toward the camera; increase speed + streak for hyperspace warp."
//! tags = ["stars", "space", "warp", "hyperspace", "galaxy"]
//! params = [
//!   { name = "speed", type = "float", default = 0.4, min = 0.0, max = 10.0 },
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 4.0 },
//!   { name = "streak", type = "float", default = 0.0, min = 0.0, max = 1.0, desc = "Motion streak length" },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "color", type = "color", default = "#cfe3ff" },
//! ]
fn layer(d: vec2f, depth: f32, p: Params) -> f32 {
    let q = d * depth * 12.0 * p.density;
    let cell = floor(q);
    let f = fract(q) - 0.5;
    let h = hash22(cell + floor(depth * 7.0));
    let pos = (h - 0.5) * 0.8;
    let r = length(f - pos);
    let size = 0.02 + 0.05 * h.x;
    let tw = 0.6 + 0.4 * sin(time() * 5.0 * h.y + h.x * 20.0);
    let dirv = normalize(d + 0.0001);
    let along = abs(dot(f - pos, vec2f(-dirv.y, dirv.x)));
    let streak = smoothstep(size, 0.0, along) * smoothstep(p.streak * 0.5 + size, 0.0, abs(dot(f - pos, dirv)));
    return max(smoothstep(size, 0.0, r) * tw, streak * p.streak);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    var v = 0.0;
    for (var i = 0; i < 4; i++) {
        let depth = fract(f32(i) * 0.25 + time() * p.speed * 0.1);
        let fade = smoothstep(0.0, 0.2, depth) * smoothstep(1.0, 0.8, depth);
        v += layer(d, mix(4.0, 0.3, depth), p) * fade;
    }
    return vec4f(p.color.rgb * v, clamp(v, 0.0, 1.0));
}
