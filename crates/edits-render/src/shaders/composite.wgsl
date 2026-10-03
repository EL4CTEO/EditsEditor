#include common
// Composites a layer (t0) onto a backdrop (t1) with blend mode, opacity, mask (t2.r) and matte.
struct Comp {
    a: vec4f,   // opacity, mode, has_mask, matte_mode (0 none, 1 alpha, 2 alpha inv, 3 luma, 4 luma inv)
    b: vec4f,   // has_matte, 0, 0, 0
};
@group(0) @binding(0) var<uniform> C: Comp;
@group(0) @binding(7) var t3: texture_2d<f32>;

fn lum(c: vec3f) -> f32 { return dot(c, vec3f(0.3, 0.59, 0.11)); }
fn clip_color(c: vec3f) -> vec3f {
    let l = lum(c);
    let n = min(c.r, min(c.g, c.b));
    let x = max(c.r, max(c.g, c.b));
    var o = c;
    if (n < 0.0) { o = l + (o - l) * l / max(l - n, 1e-5); }
    if (x > 1.0) { o = l + (o - l) * (1.0 - l) / max(x - l, 1e-5); }
    return o;
}
fn set_lum(c: vec3f, l: f32) -> vec3f { return clip_color(c + (l - lum(c))); }
fn sat(c: vec3f) -> f32 { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
fn set_sat(c: vec3f, s: f32) -> vec3f {
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    if (mx - mn <= 1e-5) { return vec3f(0.0); }
    return (c - mn) * s / (mx - mn);
}

// b = backdrop, s = source (straight, perceptual 0..1)
fn blend(b: vec3f, s: vec3f, m: i32) -> vec3f {
    switch m {
        case 2: { return b + s - b * s; }                                   // screen
        case 3: { return b * s; }                                           // multiply
        case 4: { return select(1.0 - 2.0 * (1.0 - b) * (1.0 - s), 2.0 * b * s, b <= vec3f(0.5)); } // overlay
        case 5: {                                                           // soft light
            let d = select(sqrt(b), ((16.0 * b - 12.0) * b + 4.0) * b, b <= vec3f(0.25));
            return select(b + (2.0 * s - 1.0) * (d - b), b - (1.0 - 2.0 * s) * b * (1.0 - b), s <= vec3f(0.5));
        }
        case 6: { return select(1.0 - 2.0 * (1.0 - s) * (1.0 - b), 2.0 * b * s, s <= vec3f(0.5)); } // hard light
        case 7: { return select(min(vec3f(1.0), b / max(1.0 - s, vec3f(1e-5))), vec3f(1.0), s >= vec3f(1.0)); } // dodge
        case 8: { return select(1.0 - min(vec3f(1.0), (1.0 - b) / max(s, vec3f(1e-5))), vec3f(0.0), s <= vec3f(0.0)); } // burn
        case 9: { return min(b + s, vec3f(1.0)); }                          // linear dodge
        case 10: { return max(b + s - 1.0, vec3f(0.0)); }                   // linear burn
        case 11: { return clamp(b + 2.0 * s - 1.0, vec3f(0.0), vec3f(1.0)); } // linear light
        case 12: {                                                          // vivid light
            let burn = select(1.0 - min(vec3f(1.0), (1.0 - b) / max(2.0 * s, vec3f(1e-5))), vec3f(0.0), s <= vec3f(0.0));
            let dodge = min(vec3f(1.0), b / max(2.0 * (1.0 - s), vec3f(1e-5)));
            return select(dodge, burn, s < vec3f(0.5));
        }
        case 13: { return select(max(b, 2.0 * s - 1.0), min(b, 2.0 * s), s < vec3f(0.5)); } // pin light
        case 14: { return select(vec3f(0.0), vec3f(1.0), b + s >= vec3f(1.0)); } // hard mix
        case 15: { return min(b, s); }                                      // darken
        case 16: { return max(b, s); }                                      // lighten
        case 17: { return select(b, s, lum(s) < lum(b)); }                  // darker color
        case 18: { return select(b, s, lum(s) > lum(b)); }                  // lighter color
        case 19: { return abs(b - s); }                                     // difference
        case 20: { return b + s - 2.0 * b * s; }                            // exclusion
        case 21: { return max(b - s, vec3f(0.0)); }                         // subtract
        case 22: { return min(b / max(s, vec3f(1e-5)), vec3f(1.0)); }       // divide
        case 23: { return set_lum(set_sat(s, sat(b)), lum(b)); }            // hue
        case 24: { return set_lum(set_sat(b, sat(s)), lum(b)); }            // saturation
        case 25: { return set_lum(s, lum(b)); }                             // color
        case 26: { return set_lum(b, lum(s)); }                             // luminosity
        default: { return s; }
    }
}

fn coverage(uv: vec2f) -> f32 {
    var k = C.a.x;
    if (C.a.z > 0.5) { k *= textureSampleLevel(t2, samp, uv, 0.0).r; }
    let mm = i32(C.a.w);
    if (C.b.x > 0.5 && mm > 0) {
        let m = textureSampleLevel(t3, samp, uv, 0.0);
        let l = select(0.0, dot(m.rgb / m.a, vec3f(0.2126, 0.7152, 0.0722)) * m.a, m.a > 1e-5);
        if (mm == 1) { k *= m.a; }
        else if (mm == 2) { k *= 1.0 - m.a; }
        else if (mm == 3) { k *= l; }
        else { k *= 1.0 - l; }
    }
    return k;
}

// Fast path: normal/add with hardware blending onto the backdrop.
@fragment
fn fs_source(in: VsOut) -> @location(0) vec4f {
    return textureSampleLevel(t0, samp, in.uv, 0.0) * coverage(in.uv);
}

// General path: reads the backdrop and writes the composited result.
@fragment
fn fs_blend(in: VsOut) -> @location(0) vec4f {
    let s = textureSampleLevel(t0, samp, in.uv, 0.0) * coverage(in.uv);
    let d = textureSampleLevel(t1, samp, in.uv, 0.0);
    let m = i32(C.a.y);
    if (m == 0) { return s + d * (1.0 - s.a); }
    if (m == 1) { return vec4f(s.rgb + d.rgb, min(s.a + d.a, 1.0)); }
    if (m == 27) { return d * s.a; }
    if (m == 28) { return d * (1.0 - s.a); }
    if (s.a <= 1e-5) { return d; }
    if (d.a <= 1e-5) { return s; }
    let cs = to_srgb(clamp(s.rgb / s.a, vec3f(0.0), vec3f(1.0)));
    let cb = to_srgb(clamp(d.rgb / d.a, vec3f(0.0), vec3f(1.0)));
    let bl = to_linear(clamp(blend(cb, cs, m), vec3f(0.0), vec3f(1.0)));
    let co = s.a * (1.0 - d.a) * (s.rgb / s.a) + s.a * d.a * bl + (1.0 - s.a) * d.a * (d.rgb / d.a);
    let ao = s.a + d.a * (1.0 - s.a);
    return vec4f(co, ao);
}
