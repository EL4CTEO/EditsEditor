//! id = "cross_zoom"
//! name = "Cross Zoom"
//! kind = "transition"
//! category = "motion"
//! description = "Blurred zoom crossfade (gl-transitions classic)."
//! tags = ["zoom", "blur", "crossfade", "smooth"]
//! params = [ { name = "strength", type = "float", default = 0.4, min = 0.0, max = 1.0 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let center = vec2f(mix(0.25, 0.75, ease_in_out(t)), 0.5);
    let dissolve = ease_in_out(t);
    let strength = p.strength * sin(t * PI);
    var acc = vec4f(0.0);
    var total = 0.0;
    let to_center = center - uv;
    let off = hash12(uv * res());
    for (var i = 0; i <= 24; i++) {
        let pct = (f32(i) + off) / 24.0;
        let w = 4.0 * (pct - pct * pct);
        let q = uv + to_center * pct * strength;
        acc += mix(src_clamp(q), orig_clamp(q), dissolve) * w;
        total += w;
    }
    return acc / total;
}
