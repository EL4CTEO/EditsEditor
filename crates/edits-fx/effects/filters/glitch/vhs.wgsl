//! id = "vhs"
//! name = "VHS"
//! kind = "filter"
//! category = "glitch"
//! description = "Analog VHS tape: chroma bleed, tracking jitter, noise bands, soft scanlines."
//! tags = ["vhs", "analog", "retro", "tape", "90s", "lofi"]
//! params = [
//!   { name = "intensity", type = "float", default = 0.6, min = 0.0, max = 1.0 },
//!   { name = "chroma_bleed", type = "float", default = 4.0, min = 0.0, max = 30.0 },
//!   { name = "noise", type = "float", default = 0.15, min = 0.0, max = 1.0 },
//!   { name = "tracking", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = time();
    var q = uv;
    let jitter = (vnoise(vec2f(uv.y * 200.0, t * 30.0)) - 0.5) * 0.004 * p.intensity;
    q.x += jitter;
    let band = fract(t * 0.15 + seed());
    let bd = abs(uv.y - band);
    let in_band = smoothstep(0.05, 0.0, bd) * p.tracking;
    q.x += in_band * (hash12(vec2f(floor(uv.y * 240.0), floor(t * 30.0))) - 0.5) * 0.05;
    let b = p.chroma_bleed * texel().x;
    let y = src_clamp(q);
    let r = src_clamp(q + vec2f(b, 0.0));
    let bl = src_clamp(q - vec2f(b, 0.0));
    var c = vec3f(r.r, y.g, bl.b);
    let n = hash12(uv * res() + t * 100.0);
    c += (n - 0.5) * p.noise * p.intensity;
    c += in_band * n * 0.5;
    c *= 0.92 + 0.08 * sin(uv.y * res().y * 1.2);
    c = mix(y.rgb, c, p.intensity);
    return vec4f(max(c, vec3f(0.0)), y.a);
}
