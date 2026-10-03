//! Layer placement math: fit modes, crop and the 2.5D model matrix.

use edits_core::{Fit, Mat4, TransformState};

/// Parameters for one `place` draw.
#[derive(Clone, Debug)]
pub struct PlaceParams {
    /// Column-major model matrix: layer-local pixels -> composition pixels (centered, y down).
    pub model: [[f32; 4]; 4],
    pub comp: [f32; 2],
    /// Half extents of the drawn quad (pixels, layer space).
    pub half: [f32; 2],
    pub uv_rect: [f32; 4],
    pub quad_offset: [f32; 2],
    pub perspective: f32,
    pub weight: f32,
    /// Premultiplied linear color multiplier.
    pub tint: [f32; 4],
}

/// Size of the source as placed into the composition before the transform.
pub fn fitted_size(src: (u32, u32), comp: (u32, u32), fit: Fit) -> (f64, f64) {
    let (sw, sh) = (src.0.max(1) as f64, src.1.max(1) as f64);
    let (cw, ch) = (comp.0 as f64, comp.1 as f64);
    match fit {
        Fit::Contain => {
            let s = (cw / sw).min(ch / sh);
            (sw * s, sh * s)
        }
        Fit::Cover => {
            let s = (cw / sw).max(ch / sh);
            (sw * s, sh * s)
        }
        Fit::Stretch => (cw, ch),
        Fit::None => (sw, sh),
        Fit::Width => (cw, sh * cw / sw),
        Fit::Height => (sw * ch / sh, ch),
    }
}

/// Build the model matrix for a transform state.
pub fn model_matrix(t: &TransformState, flip_x: bool, flip_y: bool) -> Mat4 {
    let sx = t.scale.x() * if flip_x { -1.0 } else { 1.0 };
    let sy = t.scale.y() * if flip_y { -1.0 } else { 1.0 };
    Mat4::translate(t.position.x(), t.position.y(), t.z)
        .mul(&Mat4::rotate_x(t.rotation_x.to_radians()))
        .mul(&Mat4::rotate_y(t.rotation_y.to_radians()))
        .mul(&Mat4::rotate_z(t.rotation.to_radians()))
        .mul(&Mat4::skew_x(t.skew.to_radians()))
        .mul(&Mat4::scale(sx, sy, 1.0))
        .mul(&Mat4::translate(-t.anchor.x(), -t.anchor.y(), 0.0))
}

/// Full placement parameters for drawing a source of `src` pixels into a `comp` canvas.
pub fn place_params(
    src: (u32, u32),
    comp: (u32, u32),
    fit: Fit,
    crop: Option<[f64; 4]>,
    state: &TransformState,
    flip: (bool, bool),
    perspective: Option<f64>,
    tint: [f32; 4],
) -> PlaceParams {
    let (bw, bh) = fitted_size(src, comp, fit);
    let [l, t, r, b] = crop.unwrap_or([0.0; 4]).map(|v| v.clamp(0.0, 1.0));
    let half = [(bw * (1.0 - l - r).max(0.0) / 2.0) as f32, (bh * (1.0 - t - b).max(0.0) / 2.0) as f32];
    let off = [(bw * (l - r) / 2.0) as f32, (bh * (t - b) / 2.0) as f32];
    let m = model_matrix(state, flip.0, flip.1);
    PlaceParams {
        model: m.to_f32(),
        comp: [comp.0 as f32, comp.1 as f32],
        half,
        uv_rect: [l as f32, t as f32, (1.0 - r) as f32, (1.0 - b) as f32],
        quad_offset: off,
        perspective: perspective.unwrap_or(comp.0 as f64 * 1.2) as f32,
        weight: 1.0,
        tint,
    }
}

/// Approximate screen-space bounding box of a placed layer (for culling), in comp pixels from
/// the top-left. Returns None if fully behind the camera.
pub fn screen_bounds(p: &PlaceParams) -> Option<[f32; 4]> {
    let m = Mat4(p.model.map(|c| c.map(|v| v as f64)));
    let f = p.perspective.max(1.0) as f64;
    let mut mn = [f32::MAX; 2];
    let mut mx = [f32::MIN; 2];
    for (cx, cy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let x = cx * p.half[0] as f64 + p.quad_offset[0] as f64;
        let y = cy * p.half[1] as f64 + p.quad_offset[1] as f64;
        let w = m.transform([x, y, 0.0]);
        let d = (f + w[2]) / f;
        if d <= 0.01 {
            return None;
        }
        let sx = (w[0] / d) as f32 + p.comp[0] / 2.0;
        let sy = (w[1] / d) as f32 + p.comp[1] / 2.0;
        mn = [mn[0].min(sx), mn[1].min(sy)];
        mx = [mx[0].max(sx), mx[1].max(sy)];
    }
    Some([mn[0], mn[1], mx[0], mx[1]])
}
