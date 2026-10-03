//! id = "zoom_through"
//! name = "Zoom Through"
//! kind = "transition"
//! category = "motion"
//! description = "Outgoing clip zooms toward camera with radial blur; incoming clip settles from a zoom. Signature AMV zoom transition."
//! tags = ["zoom", "radial blur", "impact", "amv", "fast"]
//! params = [
//!   { name = "zoom", type = "float", default = 3.0, min = 1.1, max = 20.0, desc = "Max zoom factor" },
//!   { name = "blur", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "flash", type = "float", default = 0.3, min = 0.0, max = 2.0, desc = "Brightness spike at the cut" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let n = 20;
    var acc = vec4f(0.0);
    var s = 1.0;
    var blur = 0.0;
    if (t < 0.5) {
        let k = t * 2.0;
        s = mix(1.0, p.zoom, k * k);
        blur = k * p.blur;
    } else {
        let k = (1.0 - t) * 2.0;
        s = mix(1.0, 1.0 / p.zoom, k * k);
        s = 1.0 / mix(1.0, p.zoom, k * k);
        blur = k * p.blur;
    }
    for (var i = 0; i < n; i++) {
        let f = 1.0 - blur * f32(i) / f32(n);
        let q = p.center + (uv - p.center) / (s / f);
        if (t < 0.5) { acc += src_mirror(q); } else { acc += textureSampleLevel(t1, samp, 1.0 - abs(fract(q * 0.5) * 2.0 - 1.0), 0.0); }
    }
    let c = acc / f32(n);
    let fl = exp(-pow((t - 0.5) * 8.0, 2.0)) * p.flash;
    return vec4f(c.rgb * (1.0 + fl * 3.0) + fl * 0.5, c.a);
}
