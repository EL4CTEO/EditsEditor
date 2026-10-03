#include common
// Places a source texture into a layer canvas with a full 2.5D transform (perspective-correct).
struct Place {
    model: mat4x4f,
    comp: vec4f,      // comp_w, comp_h, half_w, half_h (quad half extents, px)
    uv_rect: vec4f,   // u0, v0, u1, v1
    misc: vec4f,      // quad_off_x, quad_off_y, perspective, weight
    flags: vec4f,     // premultiply_input, 0, 0, 0
    tint: vec4f,      // premultiplied color multiplier
};
@group(0) @binding(0) var<uniform> P: Place;

struct PlaceOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_place(@builtin(vertex_index) vi: u32) -> PlaceOut {
    // triangle strip corners: (0,0) (1,0) (0,1) (1,1)
    let c = vec2f(f32(vi & 1u), f32((vi >> 1u) & 1u));
    let local = vec2f(mix(-P.comp.z, P.comp.z, c.x), mix(-P.comp.w, P.comp.w, c.y)) + P.misc.xy;
    let world = P.model * vec4f(local, 0.0, 1.0);
    let f = max(P.misc.z, 1.0);
    let w = max((f + world.z) / f, 0.01);
    var o: PlaceOut;
    o.pos = vec4f(world.x * 2.0 / P.comp.x, -world.y * 2.0 / P.comp.y, 0.0, w);
    o.pos = vec4f(o.pos.x, o.pos.y, 0.5 * w, w);
    o.uv = vec2f(mix(P.uv_rect.x, P.uv_rect.z, c.x), mix(P.uv_rect.y, P.uv_rect.w, c.y));
    return o;
}

@fragment
fn fs_place(in: PlaceOut) -> @location(0) vec4f {
    var c = textureSampleLevel(t0, samp, in.uv, 0.0);
    if (P.flags.x > 0.5) { c = vec4f(c.rgb * c.a, c.a); }
    return c * P.tint * P.misc.w;
}
