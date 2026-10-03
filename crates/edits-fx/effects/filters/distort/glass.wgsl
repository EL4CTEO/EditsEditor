//! id = "glass"
//! name = "Glass Refraction"
//! kind = "filter"
//! category = "distort"
//! description = "Frosted/rippled glass refraction with optional cells (shattered pane look)."
//! tags = ["glass", "refraction", "frosted", "crystal"]
//! params = [
//!   { name = "amount", type = "float", default = 12.0, min = 0.0, max = 100.0 },
//!   { name = "scale", type = "float", default = 30.0, min = 2.0, max = 400.0 },
//!   { name = "cells", type = "bool", default = false, desc = "Use cellular (crystal) pattern" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * res() / p.scale;
    var d = vec2f(gnoise(q), gnoise(q + 9.1));
    if (p.cells > 0.5) {
        let i = floor(q);
        d = hash22(i) * 2.0 - 1.0;
    }
    return src_clamp(uv + d * p.amount * texel());
}
