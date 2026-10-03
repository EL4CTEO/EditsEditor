#include common
// Rasterizes one mask shape (SDF) and combines it with the previous mask (t0.r).
struct Mask {
    a: vec4f,          // shape (0 rect, 1 ellipse, 2 polygon), mode (0 add 1 sub 2 inter 3 diff), invert, point_count
    b: vec4f,          // center.xy, size.xy (px, comp-centered)
    c: vec4f,          // radius, feather, expansion, opacity
    d: vec4f,          // offset.xy, rotation (rad), first (1 if first mask)
    e: vec4f,          // scale.xy, comp_w, comp_h
    pts: array<vec4f, 32>,
};
@group(0) @binding(0) var<uniform> M: Mask;

fn pt(i: i32) -> vec2f {
    let v = M.pts[i / 2];
    return select(v.zw, v.xy, (i & 1) == 0);
}

fn sd_round_box(p: vec2f, b: vec2f, r: f32) -> f32 {
    let q = abs(p) - b + r;
    return length(max(q, vec2f(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn sd_ellipse_approx(p: vec2f, ab: vec2f) -> f32 {
    let k0 = length(p / ab);
    let k1 = length(p / (ab * ab));
    return k0 * (k0 - 1.0) / max(k1, 1e-5);
}

fn sd_polygon(p: vec2f, n: i32) -> f32 {
    var d = dot(p - pt(0), p - pt(0));
    var s = 1.0;
    var j = n - 1;
    for (var i = 0; i < n; i++) {
        let vi = pt(i);
        let vj = pt(j);
        let e = vj - vi;
        let w = p - vi;
        let b = w - e * clamp(dot(w, e) / max(dot(e, e), 1e-6), 0.0, 1.0);
        d = min(d, dot(b, b));
        let c1 = p.y >= vi.y;
        let c2 = p.y < vj.y;
        let c3 = e.x * w.y > e.y * w.x;
        if ((c1 && c2 && c3) || (!c1 && !c2 && !c3)) { s = -s; }
        j = i;
    }
    return s * sqrt(d);
}

@fragment
fn fs_mask(in: VsOut) -> @location(0) vec4f {
    let px = (in.uv - 0.5) * M.e.zw;
    // inverse mask transform: offset, rotation, scale around the shape center
    var p = px - M.d.xy - M.b.xy;
    let cr = cos(-M.d.z);
    let sr = sin(-M.d.z);
    p = vec2f(p.x * cr - p.y * sr, p.x * sr + p.y * cr) / max(M.e.xy, vec2f(1e-4));
    let shape = i32(M.a.x);
    var d = 0.0;
    if (shape == 0) { d = sd_round_box(p, M.b.zw * 0.5, min(M.c.x, min(M.b.z, M.b.w) * 0.5)); }
    else if (shape == 1) { d = sd_ellipse_approx(p, max(M.b.zw * 0.5, vec2f(0.5))); }
    else { d = sd_polygon(p + M.b.xy, i32(M.a.w)); }
    d -= M.c.z;
    let f = max(M.c.y, 1.0);
    var cov = clamp(0.5 - d / f, 0.0, 1.0) * M.c.w;
    if (M.a.z > 0.5) { cov = 1.0 - cov; }
    var prev = textureSampleLevel(t0, samp, in.uv, 0.0).r;
    let mode = i32(M.a.y);
    if (M.d.w > 0.5) { prev = select(1.0, 0.0, mode == 0 || mode == 3); }
    var o = prev;
    if (mode == 0) { o = prev + cov * (1.0 - prev); }
    else if (mode == 1) { o = prev * (1.0 - cov); }
    else if (mode == 2) { o = prev * cov; }
    else { o = abs(prev - cov); }
    return vec4f(o, 0.0, 0.0, 1.0);
}
