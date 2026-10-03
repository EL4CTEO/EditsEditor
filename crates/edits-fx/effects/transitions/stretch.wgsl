//! id = "stretch"
//! name = "Stretch"
//! kind = "transition"
//! category = "motion"
//! description = "Outgoing clip stretches into a line, incoming clip springs out of it."
//! tags = ["stretch", "squash", "cartoon", "snap"]
//! params = [
//!   { name = "vertical", type = "bool", default = false },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    var q = uv - 0.5;
    if (t < 0.5) {
        let k = t * 2.0;
        let s = mix(1.0, 0.002, k * k);
        if (p.vertical > 0.5) { q.x /= s; q.y /= mix(1.0, 3.0, k); } else { q.y /= s; q.x /= mix(1.0, 3.0, k); }
        return src(q + 0.5);
    }
    let k = (1.0 - t) * 2.0;
    let s = mix(1.0, 0.002, k * k);
    if (p.vertical > 0.5) { q.x /= s; q.y /= mix(1.0, 3.0, k); } else { q.y /= s; q.x /= mix(1.0, 3.0, k); }
    return orig(q + 0.5);
}
