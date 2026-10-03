//! id = "data_mosh"
//! name = "Datamosh Smear"
//! kind = "filter"
//! category = "glitch"
//! description = "Blocky motion-vector style smearing driven by noise, imitating compressed-video datamoshing."
//! tags = ["datamosh", "glitch", "compression", "smear"]
//! params = [
//!   { name = "amount", type = "float", default = 40.0, min = 0.0, max = 400.0 },
//!   { name = "block", type = "float", default = 16.0, min = 4.0, max = 128.0 },
//!   { name = "speed", type = "float", default = 6.0, min = 0.0, max = 60.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let b = floor(uv * res() / p.block);
    let t = floor(time() * p.speed);
    let mv = (hash22(b + t) - 0.5) * 2.0;
    let strength = step(0.6, hash12(b * 0.37 + t));
    return src_clamp(uv + mv * p.amount * texel() * strength);
}
