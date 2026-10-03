//! Vector shapes (kurbo geometry + tiny-skia rasterization) with gradients, dashes and trim
//! paths (stroke draw-on animations). Also SVG-path flattening for masks.

use edits_core::{Color, Fill, GradientKind, LineCap, ShapeKind, ShapeSource};
use edits_media::Frame;
use kurbo::{BezPath, ParamCurve, ParamCurveArclen, PathEl, Point, Shape};

/// Geometry of a shape kind, centered on (0, 0).
pub fn shape_path(kind: &ShapeKind) -> BezPath {
    match kind {
        ShapeKind::Rect { size, radius } => {
            let r = kurbo::Rect::new(-size.x() / 2.0, -size.y() / 2.0, size.x() / 2.0, size.y() / 2.0);
            if *radius > 0.0 { r.to_rounded_rect(radius.min(size.x().min(size.y()) / 2.0)).to_path(0.1) } else { r.to_path(0.1) }
        }
        ShapeKind::Ellipse { size } => kurbo::Ellipse::new((0.0, 0.0), (size.x() / 2.0, size.y() / 2.0), 0.0).to_path(0.1),
        ShapeKind::Polygon { sides, radius } => {
            let n = (*sides).max(3);
            let mut p = BezPath::new();
            for i in 0..n {
                let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / n as f64;
                let pt = Point::new(a.cos() * radius, a.sin() * radius);
                if i == 0 { p.move_to(pt) } else { p.line_to(pt) }
            }
            p.close_path();
            p
        }
        ShapeKind::Star { points, outer_radius, inner_radius } => {
            let n = (*points).max(2) * 2;
            let mut p = BezPath::new();
            for i in 0..n {
                let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / n as f64;
                let r = if i % 2 == 0 { *outer_radius } else { *inner_radius };
                let pt = Point::new(a.cos() * r, a.sin() * r);
                if i == 0 { p.move_to(pt) } else { p.line_to(pt) }
            }
            p.close_path();
            p
        }
        ShapeKind::Line { from, to } => {
            let mut p = BezPath::new();
            p.move_to((from.x(), from.y()));
            p.line_to((to.x(), to.y()));
            p
        }
        ShapeKind::Ring { outer_radius, inner_radius } => {
            let mut p = kurbo::Circle::new((0.0, 0.0), *outer_radius).to_path(0.1);
            let inner = kurbo::Circle::new((0.0, 0.0), *inner_radius).to_path(0.1);
            p.extend(inner.iter());
            p
        }
        ShapeKind::Path { d } => BezPath::from_svg(d).unwrap_or_default(),
    }
}

/// Keep the [start, end] fraction (0..1) of the path's length, rotated by `offset`.
pub fn trim_path(path: &BezPath, start: f64, end: f64, offset: f64) -> BezPath {
    let (mut a, mut b) = (start.clamp(0.0, 1.0), end.clamp(0.0, 1.0));
    if a > b {
        std::mem::swap(&mut a, &mut b);
    }
    if a <= 0.0 && b >= 1.0 && offset == 0.0 {
        return path.clone();
    }
    let segs: Vec<kurbo::PathSeg> = path.segments().collect();
    let lens: Vec<f64> = segs.iter().map(|s| s.arclen(0.05)).collect();
    let total: f64 = lens.iter().sum();
    if total <= 0.0 {
        return BezPath::new();
    }
    let mut out = BezPath::new();
    let emit = |from: f64, to: f64, out: &mut BezPath| {
        // from/to in absolute length within [0, total]
        let mut acc = 0.0;
        let mut started = false;
        for (seg, len) in segs.iter().zip(lens.iter()) {
            let (s0, s1) = (acc, acc + len);
            acc = s1;
            if s1 <= from || s0 >= to || *len <= 0.0 {
                continue;
            }
            let t0 = if from > s0 { seg.inv_arclen(from - s0, 0.05) } else { 0.0 };
            let t1 = if to < s1 { seg.inv_arclen(to - s0, 0.05) } else { 1.0 };
            let sub = seg.subsegment(t0..t1);
            if !started {
                out.move_to(sub.start());
                started = true;
            }
            match sub {
                kurbo::PathSeg::Line(l) => out.line_to(l.p1),
                kurbo::PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
                kurbo::PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
            }
        }
    };
    let off = offset.rem_euclid(1.0) * total;
    let (fa, fb) = (a * total + off, b * total + off);
    if fb <= total {
        emit(fa, fb, &mut out);
    } else if fa >= total {
        emit(fa - total, fb - total, &mut out);
    } else {
        emit(fa, total, &mut out);
        emit(0.0, fb - total, &mut out);
    }
    out
}

fn to_skia(path: &BezPath, dx: f64, dy: f64) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => pb.move_to((p.x + dx) as f32, (p.y + dy) as f32),
            PathEl::LineTo(p) => pb.line_to((p.x + dx) as f32, (p.y + dy) as f32),
            PathEl::QuadTo(a, b) => pb.quad_to((a.x + dx) as f32, (a.y + dy) as f32, (b.x + dx) as f32, (b.y + dy) as f32),
            PathEl::CurveTo(a, b, c) => pb.cubic_to(
                (a.x + dx) as f32,
                (a.y + dy) as f32,
                (b.x + dx) as f32,
                (b.y + dy) as f32,
                (c.x + dx) as f32,
                (c.y + dy) as f32,
            ),
            PathEl::ClosePath => pb.close(),
        }
    }
    pb.finish()
}

fn skia_color(c: &Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(c.0[0].clamp(0.0, 1.0), c.0[1].clamp(0.0, 1.0), c.0[2].clamp(0.0, 1.0), c.0[3].clamp(0.0, 1.0)).unwrap_or(tiny_skia::Color::WHITE)
}

/// Rasterize a shape. The returned frame is centered on the shape origin (0, 0).
pub fn render_shape(src: &ShapeSource, trim: Option<(f64, f64, f64)>) -> Frame {
    let mut path = shape_path(&src.shape);
    let stroke_w = src.stroke.as_ref().map(|s| s.width).unwrap_or(0.0);
    let bbox = path.bounding_box();
    if let Some((a, b, o)) = trim {
        path = trim_path(&path, a, b, o);
    }
    let pad = stroke_w / 2.0 + 3.0;
    let hx = bbox.x0.abs().max(bbox.x1.abs()) + pad;
    let hy = bbox.y0.abs().max(bbox.y1.abs()) + pad;
    let (w, h) = (((hx * 2.0).ceil() as u32).clamp(1, 8192), ((hy * 2.0).ceil() as u32).clamp(1, 8192));
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    let (dx, dy) = (w as f64 / 2.0, h as f64 / 2.0);
    if let Some(sp) = to_skia(&path, dx, dy) {
        let trimmed = trim.is_some_and(|(a, b, _)| a > 0.0 || b < 1.0);
        if let Some(fill) = &src.fill {
            if !trimmed {
                let mut paint = tiny_skia::Paint { anti_alias: true, ..Default::default() };
                match fill {
                    Fill::Solid(c) => paint.set_color(skia_color(c)),
                    Fill::Gradient(g) => {
                        let stops: Vec<tiny_skia::GradientStop> =
                            g.stops.iter().map(|(o, c)| tiny_skia::GradientStop::new(*o as f32, skia_color(c))).collect();
                        let shader = match g.kind {
                            GradientKind::Linear => {
                                let a = g.angle.to_radians();
                                let (cx, cy) = (a.cos() * bbox.width() / 2.0, a.sin() * bbox.height() / 2.0);
                                tiny_skia::LinearGradient::new(
                                    tiny_skia::Point::from_xy((dx - cx) as f32, (dy - cy) as f32),
                                    tiny_skia::Point::from_xy((dx + cx) as f32, (dy + cy) as f32),
                                    stops,
                                    tiny_skia::SpreadMode::Pad,
                                    tiny_skia::Transform::identity(),
                                )
                            }
                            GradientKind::Radial => tiny_skia::RadialGradient::new(
                                tiny_skia::Point::from_xy(dx as f32, dy as f32),
                                0.0,
                                tiny_skia::Point::from_xy(dx as f32, dy as f32),
                                (bbox.width().max(bbox.height()) / 2.0) as f32,
                                stops,
                                tiny_skia::SpreadMode::Pad,
                                tiny_skia::Transform::identity(),
                            ),
                        };
                        if let Some(s) = shader {
                            paint.shader = s;
                        }
                    }
                }
                let rule = if matches!(src.shape, ShapeKind::Ring { .. }) { tiny_skia::FillRule::EvenOdd } else { tiny_skia::FillRule::Winding };
                pm.fill_path(&sp, &paint, rule, tiny_skia::Transform::identity(), None);
            }
        }
        if let Some(st) = &src.stroke {
            if st.width > 0.0 {
                let mut paint = tiny_skia::Paint { anti_alias: true, ..Default::default() };
                paint.set_color(skia_color(&st.color));
                let mut stroke = tiny_skia::Stroke {
                    width: st.width as f32,
                    line_cap: match st.cap {
                        LineCap::Round => tiny_skia::LineCap::Round,
                        LineCap::Butt => tiny_skia::LineCap::Butt,
                        LineCap::Square => tiny_skia::LineCap::Square,
                    },
                    line_join: tiny_skia::LineJoin::Round,
                    ..Default::default()
                };
                if st.dash.len() >= 2 {
                    stroke.dash = tiny_skia::StrokeDash::new(st.dash.iter().map(|d| *d as f32).collect(), 0.0);
                }
                pm.stroke_path(&sp, &paint, &stroke, tiny_skia::Transform::identity(), None);
            }
        }
    }
    Frame::new(w, h, pm.take(), true)
}

/// Flatten an SVG path to a polygon (for masks), at most `max_points` points.
pub fn flatten_svg(d: &str, max_points: usize) -> Vec<[f32; 2]> {
    let Ok(p) = BezPath::from_svg(d) else { return vec![] };
    let mut pts = vec![];
    let mut tol = 0.5;
    loop {
        pts.clear();
        kurbo::flatten(p.iter(), tol, |el| match el {
            PathEl::MoveTo(q) | PathEl::LineTo(q) => pts.push([q.x as f32, q.y as f32]),
            _ => {}
        });
        if pts.len() <= max_points || tol > 64.0 {
            break;
        }
        tol *= 2.0;
    }
    pts.truncate(max_points);
    pts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_half_length() {
        let p = shape_path(&ShapeKind::Line { from: edits_core::Vec2::new(0.0, 0.0), to: edits_core::Vec2::new(100.0, 0.0) });
        let t = trim_path(&p, 0.0, 0.5, 0.0);
        let len: f64 = t.segments().map(|s| s.arclen(0.01)).sum();
        assert!((len - 50.0).abs() < 0.5);
    }

    #[test]
    fn renders_star() {
        let s = ShapeSource {
            shape: ShapeKind::Star { points: 5, outer_radius: 50.0, inner_radius: 20.0 },
            fill: Some(Fill::Solid(Color::WHITE)),
            stroke: None,
            trim: None,
        };
        let f = render_shape(&s, None);
        assert!(f.width >= 100);
        let center = ((f.height / 2) * f.width + f.width / 2) as usize * 4;
        assert!(f.data[center + 3] > 200);
    }
}
