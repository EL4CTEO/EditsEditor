// ===================== EditsEditor effect prelude =====================
// Color space: linear-light, premultiplied alpha RGBA (rgba16float).
// Color params arrive as linear RGB with straight alpha in .a.
// uv: (0,0) = top-left, (1,1) = bottom-right of the output.

struct Globals {
    resolution: vec2f,   // output size in pixels
    time: f32,           // composition time (s)
    local_time: f32,     // clip-local time (s)
    progress: f32,       // transitions: 0..1 eased progress; others: local_time / duration
    duration: f32,       // clip (or transition) duration (s)
    frame: f32,          // composition frame index
    seed: f32,           // per-instance random seed
    beat_time: f32,      // seconds since the last beat (large if none)
    beat_index: f32,     // index of the last beat (-1 if none)
    bpm: f32,            // tempo (0 if unknown)
    audio_level: f32,    // overall loudness 0..1 at this instant
    audio_bands: vec4f,  // bass, low-mid, high-mid, treble 0..1
    pass_data: vec4f,    // per-pass constants from the effect header
    texel: vec2f,        // 1 / resolution
    src_resolution: vec2f, // size of input texture t0
};

struct Uniforms {
    g: Globals,
    p: array<vec4f, 32>,
};

@group(0) @binding(0) var<uniform> U: Uniforms;
@group(0) @binding(1) var samp: sampler;          // linear, clamp-to-edge
@group(0) @binding(2) var t0: texture_2d<f32>;     // main input (previous pass / transition "from")
@group(0) @binding(3) var t1: texture_2d<f32>;     // original effect input / transition "to"
@group(0) @binding(4) var t2: texture_2d<f32>;     // extra input (named pass output, image/LUT param)
@group(0) @binding(5) var samp_wrap: sampler;     // linear, repeat
@group(0) @binding(6) var samp_near: sampler;     // nearest, clamp

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;
const LUMA: vec3f = vec3f(0.2126, 0.7152, 0.0722);

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var o: VsOut;
    let x = f32((vi << 1u) & 2u);
    let y = f32(vi & 2u);
    o.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2f(x, y);
    return o;
}

// ---------- globals shortcuts ----------
fn res() -> vec2f { return U.g.resolution; }
fn texel() -> vec2f { return U.g.texel; }
fn aspect() -> f32 { return U.g.resolution.x / max(U.g.resolution.y, 1.0); }
fn time() -> f32 { return U.g.time; }
fn ltime() -> f32 { return U.g.local_time; }
fn progress() -> f32 { return U.g.progress; }
fn seed() -> f32 { return U.g.seed; }
fn bass() -> f32 { return U.g.audio_bands.x; }
fn treble() -> f32 { return U.g.audio_bands.w; }
/// exp-decaying pulse on every beat (1 at the beat, fades with `decay` per second)
fn beat_pulse(decay: f32) -> f32 { return exp(-U.g.beat_time * decay); }

// ---------- sampling ----------
fn inside(uv: vec2f) -> f32 {
    return select(0.0, 1.0, all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0)));
}
/// Input sample, transparent outside the frame.
fn src(uv: vec2f) -> vec4f { return textureSampleLevel(t0, samp, uv, 0.0) * inside(uv); }
/// Input sample with edge extension.
fn src_clamp(uv: vec2f) -> vec4f { return textureSampleLevel(t0, samp, uv, 0.0); }
/// Input sample tiled.
fn src_wrap(uv: vec2f) -> vec4f { return textureSampleLevel(t0, samp_wrap, uv, 0.0); }
/// Input sample mirrored at the edges.
fn src_mirror(uv: vec2f) -> vec4f {
    let m = abs(fract(uv * 0.5) * 2.0 - 1.0);
    return textureSampleLevel(t0, samp, 1.0 - m, 0.0);
}
/// Edge handling: 0 transparent, 1 clamp, 2 mirror, 3 wrap.
fn src_edge(uv: vec2f, mode: f32) -> vec4f {
    let m = i32(mode + 0.5);
    if (m == 1) { return src_clamp(uv); }
    if (m == 2) { return src_mirror(uv); }
    if (m == 3) { return src_wrap(uv); }
    return src(uv);
}
fn src_near(uv: vec2f) -> vec4f { return textureSampleLevel(t0, samp_near, uv, 0.0) * inside(uv); }
fn orig(uv: vec2f) -> vec4f { return textureSampleLevel(t1, samp, uv, 0.0) * inside(uv); }
fn orig_clamp(uv: vec2f) -> vec4f { return textureSampleLevel(t1, samp, uv, 0.0); }
fn extra(uv: vec2f) -> vec4f { return textureSampleLevel(t2, samp, uv, 0.0); }
fn extra_wrap(uv: vec2f) -> vec4f { return textureSampleLevel(t2, samp_wrap, uv, 0.0); }
/// Transition helpers: "from" (outgoing) and "to" (incoming) images.
fn from_img(uv: vec2f) -> vec4f { return src(uv); }
fn to_img(uv: vec2f) -> vec4f { return orig(uv); }

// ---------- color ----------
fn luma(c: vec3f) -> f32 { return dot(c, LUMA); }
fn unpremul(c: vec4f) -> vec4f {
    if (c.a <= 0.00001) { return vec4f(0.0); }
    return vec4f(c.rgb / c.a, c.a);
}
fn premul(c: vec4f) -> vec4f { return vec4f(c.rgb * c.a, c.a); }
fn to_srgb(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}
fn to_linear(c: vec3f) -> vec3f {
    let lo = c / 12.92;
    let hi = pow(max((c + 0.055) / 1.055, vec3f(0.0)), vec3f(2.4));
    return select(hi, lo, c <= vec3f(0.04045));
}
fn rgb2hsv(c: vec3f) -> vec3f {
    let k = vec4f(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    let p = mix(vec4f(c.bg, k.wz), vec4f(c.gb, k.xy), step(c.b, c.g));
    let q = mix(vec4f(p.xyw, c.r), vec4f(c.r, p.yzx), step(p.x, c.r));
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3f(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}
fn hsv2rgb(c: vec3f) -> vec3f {
    let k = vec4f(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + k.xyz) * 6.0 - k.www);
    return c.z * mix(k.xxx, clamp(p - k.xxx, vec3f(0.0), vec3f(1.0)), c.y);
}
fn saturate3(c: vec3f) -> vec3f { return clamp(c, vec3f(0.0), vec3f(1.0)); }
/// Apply a straight-alpha color transform to a premultiplied pixel in perceptual (sRGB) space.
fn grade_begin(c: vec4f) -> vec4f { let u = unpremul(c); return vec4f(to_srgb(u.rgb), u.a); }
fn grade_end(c: vec4f) -> vec4f { return premul(vec4f(to_linear(max(c.rgb, vec3f(0.0))), c.a)); }
fn over(top: vec4f, bottom: vec4f) -> vec4f { return top + bottom * (1.0 - top.a); }

// ---------- math ----------
fn rot2(a: f32) -> mat2x2f { let c = cos(a); let s = sin(a); return mat2x2f(c, s, -s, c); }
/// Rotate/scale a uv around a center, aspect-correct.
fn uv_transform(uv: vec2f, center: vec2f, angle: f32, scale: f32) -> vec2f {
    var d = (uv - center) * vec2f(aspect(), 1.0);
    d = rot2(angle) * d / max(scale, 0.0001);
    return d / vec2f(aspect(), 1.0) + center;
}
fn remap(v: f32, a: f32, b: f32, c: f32, d: f32) -> f32 { return c + (v - a) * (d - c) / (b - a); }
fn ease_in_out(t: f32) -> f32 { return t * t * (3.0 - 2.0 * t); }
fn ease_out_expo(t: f32) -> f32 { return select(1.0 - pow(2.0, -10.0 * t), 1.0, t >= 1.0); }

fn fmod_pos(x: f32, y: f32) -> f32 { return x - y * floor(x / y); }
fn fmod_v2(v: vec2f, y: f32) -> vec2f { return v - y * floor(v / y); }
fn screen3(a: vec3f, b: vec3f) -> vec3f { return 1.0 - (1.0 - a) * (1.0 - b); }
/// Direction vector for an angle in degrees (0 = right, 90 = down).
fn dir_deg(deg: f32) -> vec2f { let a = radians(deg); return vec2f(cos(a), sin(a)); }
/// Mix original input (t1) and processed color by amount.
fn with_mix(processed: vec4f, uv: vec2f, amount: f32) -> vec4f { return mix(orig(uv), processed, amount); }

// ---------- hashing & noise ----------
fn hash11(p: f32) -> f32 { var x = fract(p * 0.1031); x *= x + 33.33; x *= x + x; return fract(x); }
fn hash12(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}
fn hash22(p: vec2f) -> vec2f {
    var p3 = fract(vec3f(p.xyx) * vec3f(0.1031, 0.1030, 0.0973));
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.xx + p3.yz) * p3.zy);
}
fn hash33(p: vec3f) -> vec3f {
    var q = fract(p * vec3f(0.1031, 0.1030, 0.0973));
    q += dot(q, q.yxz + 33.33);
    return fract((q.xxy + q.yxx) * q.zyx);
}
fn vnoise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash12(i);
    let b = hash12(i + vec2f(1.0, 0.0));
    let c = hash12(i + vec2f(0.0, 1.0));
    let d = hash12(i + vec2f(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
/// Gradient noise in -1..1
fn gnoise(p: vec2f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let ga = hash22(i) * 2.0 - 1.0;
    let gb = hash22(i + vec2f(1.0, 0.0)) * 2.0 - 1.0;
    let gc = hash22(i + vec2f(0.0, 1.0)) * 2.0 - 1.0;
    let gd = hash22(i + vec2f(1.0, 1.0)) * 2.0 - 1.0;
    let va = dot(ga, f);
    let vb = dot(gb, f - vec2f(1.0, 0.0));
    let vc = dot(gc, f - vec2f(0.0, 1.0));
    let vd = dot(gd, f - vec2f(1.0, 1.0));
    return mix(mix(va, vb, u.x), mix(vc, vd, u.x), u.y) * 1.4142;
}
fn fbm(p: vec2f, octaves: i32) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < octaves; i++) {
        v += a * gnoise(q);
        q = rot2(0.5) * q * 2.03 + vec2f(1.7, 9.2);
        a *= 0.5;
    }
    return v;
}
/// Worley/cellular noise: returns (distance to nearest, distance to second nearest)
fn worley(p: vec2f) -> vec2f {
    let i = floor(p);
    let f = fract(p);
    var d1 = 8.0;
    var d2 = 8.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let o = vec2f(f32(x), f32(y));
            let r = o + hash22(i + o) - f;
            let d = dot(r, r);
            if (d < d1) { d2 = d1; d1 = d; } else if (d < d2) { d2 = d; }
        }
    }
    return sqrt(vec2f(d1, d2));
}
/// Smooth 1D noise over time, -1..1 (for shakes / wiggles)
fn noise1(t: f32) -> f32 {
    let i = floor(t);
    let f = fract(t);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(hash11(i) * 2.0 - 1.0, hash11(i + 1.0) * 2.0 - 1.0, u);
}

// ---------- shapes ----------
fn sd_box(p: vec2f, b: vec2f) -> f32 { let d = abs(p) - b; return length(max(d, vec2f(0.0))) + min(max(d.x, d.y), 0.0); }
fn sd_circle(p: vec2f, r: f32) -> f32 { return length(p) - r; }
/// Aspect-corrected position relative to `center` in uv space (y units).
fn centered(uv: vec2f, center: vec2f) -> vec2f { return (uv - center) * vec2f(aspect(), 1.0); }
/// Anti-aliased step for distances measured in uv-y units (as returned by `centered`).
fn aa_step(edge: f32, x: f32) -> f32 { let w = 1.25 * U.g.texel.y; return smoothstep(edge - w, edge + w, x); }

// ---------- blur helpers ----------
/// Separable gaussian-ish blur along `dir` (pixels) with `radius` pixels. Uses linear filtering taps.
fn blur_dir(uv: vec2f, dir: vec2f, radius: f32) -> vec4f {
    let r = max(radius, 0.0);
    if (r < 0.5) { return src_clamp(uv); }
    let taps = i32(clamp(ceil(r / 1.5), 2.0, 48.0));
    let step_px = r / f32(taps);
    let sigma = r * 0.45;
    var acc = vec4f(0.0);
    var wsum = 0.0;
    for (var i = -taps; i <= taps; i++) {
        let x = f32(i) * step_px;
        let w = exp(-(x * x) / (2.0 * sigma * sigma));
        acc += src_clamp(uv + dir * x * texel()) * w;
        wsum += w;
    }
    return acc / wsum;
}
// ===================== end prelude =====================
