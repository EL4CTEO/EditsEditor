//! id = "strobe"
//! name = "Strobe"
//! kind = "filter"
//! category = "light"
//! description = "Strobing flashes at a fixed rate or locked to the beat."
//! tags = ["strobe", "flash", "flicker", "beat", "rave"]
//! params = [
//!   { name = "rate", type = "float", default = 8.0, min = 0.5, max = 30.0, desc = "Flashes per second (ignored when on_beat)" },
//!   { name = "on_beat", type = "bool", default = false, desc = "Flash on each music beat" },
//!   { name = "duty", type = "float", default = 0.5, min = 0.05, max = 0.95, desc = "Fraction of the cycle that is lit" },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "amount", type = "float", default = 0.8, min = 0.0, max = 1.0 },
//!   { name = "invert", type = "bool", default = false, desc = "Invert colors instead of flashing" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var on = 0.0;
    if (p.on_beat > 0.5) {
        on = select(0.0, 1.0, U.g.beat_time < (60.0 / max(U.g.bpm, 1.0)) * p.duty * 0.5);
    } else {
        on = select(0.0, 1.0, fract(time() * p.rate) < p.duty);
    }
    let c = src_clamp(uv);
    if (p.invert > 0.5) {
        let u = unpremul(c);
        return mix(c, premul(vec4f(1.0 - u.rgb, u.a)), on * p.amount);
    }
    return mix(c, vec4f(p.color.rgb, 1.0) * max(c.a, p.color.a), on * p.amount);
}
