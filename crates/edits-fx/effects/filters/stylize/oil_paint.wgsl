//! id = "oil_paint"
//! name = "Oil Paint (Kuwahara)"
//! kind = "filter"
//! category = "stylize"
//! description = "Painterly Kuwahara filter: flat color regions with preserved edges (watercolor/anime background look)."
//! tags = ["oil paint", "painterly", "kuwahara", "watercolor", "art"]
//! params = [
//!   { name = "radius", type = "float", default = 5.0, min = 1.0, max = 12.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let r = i32(clamp(p.radius, 1.0, 12.0));
    var m = array<vec3f, 4>(vec3f(0.0), vec3f(0.0), vec3f(0.0), vec3f(0.0));
    var s = array<vec3f, 4>(vec3f(0.0), vec3f(0.0), vec3f(0.0), vec3f(0.0));
    var off = array<vec2i, 4>(vec2i(-1, -1), vec2i(1, -1), vec2i(-1, 1), vec2i(1, 1));
    let n = f32((r + 1) * (r + 1));
    for (var k = 0; k < 4; k++) {
        for (var j = 0; j <= r; j++) {
            for (var i = 0; i <= r; i++) {
                let o = vec2f(f32(i * off[k].x), f32(j * off[k].y));
                let c = src_clamp(uv + o * texel()).rgb;
                m[k] += c;
                s[k] += c * c;
            }
        }
    }
    var best = vec3f(0.0);
    var minv = 1e9;
    for (var k = 0; k < 4; k++) {
        let mean = m[k] / n;
        let v = abs(s[k] / n - mean * mean);
        let vv = v.r + v.g + v.b;
        if (vv < minv) { minv = vv; best = mean; }
    }
    return vec4f(best, src_clamp(uv).a);
}
