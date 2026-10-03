//! id = "cube"
//! name = "3D Cube"
//! kind = "transition"
//! category = "geometric"
//! description = "Rotating 3D cube with perspective."
//! tags = ["cube", "3d", "rotate", "perspective"]
//! params = [
//!   { name = "perspective", type = "float", default = 0.7, min = 0.0, max = 1.0 },
//!   { name = "unzoom", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "floor_reflect", type = "float", default = 0.0, min = 0.0, max = 1.0 },
//! ]
fn persp(p0: vec2f, persp_amt: f32, center: f32) -> vec2f {
    let x2 = (p0.x * 2.0 - 1.0);
    let y = (p0.y - 0.5) * (1.0 - persp_amt * (1.0 - abs(center)) * 0.0) ;
    return vec2f(p0.x, y + 0.5);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let ang = t * PI * 0.5;
    let z = 1.0 + sin(t * PI) * p.unzoom;
    let q = (uv - 0.5) * z;
    // two faces: from rotates away to the left, to comes from the right
    let w_from = cos(ang);
    let w_to = sin(ang);
    let split = w_from - 0.5 * (w_from + w_to) + 0.5 * (w_from - w_to) * 0.0;
    let edge = -0.5 + w_from;
    var c = vec4f(0.0);
    let x = q.x + 0.5;
    let bound = w_from;
    if (x < bound) {
        let fx = x / max(bound, 0.0001);
        let sc = 1.0 + (1.0 - fx) * p.perspective * sin(ang) * 0.5;
        let fy = q.y / sc + 0.5;
        c = from_img(vec2f(fx, fy));
    } else {
        let tx = (x - bound) / max(1.0 - bound, 0.0001);
        let sc = 1.0 + tx * p.perspective * cos(ang) * 0.5;
        let ty = q.y / sc + 0.5;
        c = to_img(vec2f(tx, ty));
    }
    return c;
}
