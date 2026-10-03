//! id = "energy_aura"
//! name = "Energy Aura"
//! kind = "generator"
//! category = "energy"
//! description = "Flickering power-up aura flames radiating from an elliptical core (super saiyan vibes)."
//! tags = ["aura", "energy", "power up", "anime", "flame"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.6] },
//!   { name = "size", type = "vec2", default = [0.18, 0.35], desc = "Core ellipse radii (frame heights)" },
//!   { name = "color", type = "color", default = "#ffd23f" },
//!   { name = "intensity", type = "float", default = 1.5, min = 0.0, max = 5.0 },
//!   { name = "speed", type = "float", default = 1.5, min = 0.0, max = 10.0 },
//!   { name = "spread", type = "float", default = 0.25, min = 0.01, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center) / max(p.size, vec2f(0.001));
    let r = length(d);
    let a = atan2(d.y, d.x);
    let t = time() * p.speed;
    let n = fbm(vec2f(a * 3.0, r * 2.0 - t * 2.0), 5) * 0.5 + 0.5;
    let flame = smoothstep(1.0 + p.spread * 4.0 * n, 0.9, r) * (0.6 + n);
    let up = smoothstep(0.5, -1.0, d.y) * 0.5 + 0.5;
    let v = flame * up * p.intensity;
    let c = mix(p.color.rgb, vec3f(1.0), smoothstep(0.6, 1.5, v)) * v;
    return vec4f(c, clamp(v, 0.0, 1.0));
}
