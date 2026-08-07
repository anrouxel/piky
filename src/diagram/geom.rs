//! Plain 2D point math shared by every diagram type's edge routing and
//! hit-testing (polyline distance tests, marker direction vectors, ...).

use merman_render::model::LayoutPoint;

pub type Pt = (f64, f64);

pub fn pt(p: &LayoutPoint) -> Pt {
    (p.x, p.y)
}

pub fn add(a: Pt, b: Pt) -> Pt {
    (a.0 + b.0, a.1 + b.1)
}

pub fn sub(a: Pt, b: Pt) -> Pt {
    (a.0 - b.0, a.1 - b.1)
}

pub fn scale(a: Pt, s: f64) -> Pt {
    (a.0 * s, a.1 * s)
}

pub fn dist(a: Pt, b: Pt) -> f64 {
    let d = sub(a, b);
    (d.0 * d.0 + d.1 * d.1).sqrt()
}

pub fn normalize(v: Pt) -> Pt {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len <= 1e-9 { (0.0, 1.0) } else { (v.0 / len, v.1 / len) }
}

fn point_segment_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let d = sub(b, a);
    let len2 = d.0 * d.0 + d.1 * d.1;
    if len2 <= 1e-9 {
        return dist(p, a);
    }
    let t = (((p.0 - a.0) * d.0 + (p.1 - a.1) * d.1) / len2).clamp(0.0, 1.0);
    dist(p, add(a, scale(d, t)))
}

/// Hit-test tolerance is checked against the straight polyline through the
/// layout points rather than the smoothed curve drawn on screen; the curve
/// stays close enough to its control points that this is not noticeable at
/// the pixel tolerances click hit-testing uses.
pub fn polyline_hit(x: f64, y: f64, points: &[LayoutPoint], tolerance: f64) -> bool {
    points
        .windows(2)
        .any(|seg| point_segment_distance((x, y), pt(&seg[0]), pt(&seg[1])) <= tolerance)
}
