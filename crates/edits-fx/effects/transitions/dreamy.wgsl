//! id = "dreamy"
//! name = "Dreamy"
//! kind = "transition"
//! category = "organic"
//! description = "Soft wavy blur dissolve (memories, flashbacks)."
//! tags = ["dreamy", "flashback", "soft", "wavy", "memory"]
//! params = [ { name = "glow", type = "float", default = 0.4, min = 0.0, max = 2.0 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let k = sin(t * PI);
    let off = vec2f(sin(uv.y * 12.0 + t * 8.0), cos(uv.x * 10.0 + t * 6.0)) * 0.01 * k;
    var acc = vec4f(0.0);
    for (var i = 0; i < 12; i++) {
        let a = f32(i) * 2.39996;
        let r = sqrt(f32(i) / 12.0) * 12.0 * k;
        let q = uv + off + vec2f(cos(a), sin(a)) * r * texel();
        acc += mix(src_clamp(q), orig_clamp(q), t);
    }
    let c = acc / 12.0;
    return vec4f(c.rgb * (1.0 + k * p.glow), c.a);
}
