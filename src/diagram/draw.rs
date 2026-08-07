//! Backend-agnostic-per-diagram-type GSK drawing primitives: rects, polygons,
//! smoothed multi-point curves, and Pango text. Shared by every diagram
//! type's drawing code so the GSK/Pango mechanics live in exactly one place.

use gtk::gdk;
use gtk::graphene;
use gtk::gsk;
use gtk::pango;
use gtk::prelude::*;

use merman_render::model::LayoutPoint;

use super::geom::Pt;

#[derive(Clone, Copy)]
pub enum HAlign {
    Center,
    Left,
}

pub fn fill_rect(snapshot: &gtk::Snapshot, rect: &graphene::Rect, color: &gdk::RGBA) {
    snapshot.append_color(color, rect);
}

pub fn stroke_rect(
    snapshot: &gtk::Snapshot,
    rect: &graphene::Rect,
    color: &gdk::RGBA,
    width: f32,
    dash: Option<&[f32]>,
) {
    let builder = gsk::PathBuilder::new();
    builder.add_rect(rect);
    let path = builder.to_path();
    let stroke = gsk::Stroke::new(width);
    if let Some(dash) = dash {
        stroke.set_dash(dash);
    }
    snapshot.append_stroke(&path, &stroke, color);
}

pub fn stroke_hline(snapshot: &gtk::Snapshot, x0: f64, x1: f64, y: f64, color: &gdk::RGBA, width: f32) {
    let builder = gsk::PathBuilder::new();
    builder.move_to(x0 as f32, y as f32);
    builder.line_to(x1 as f32, y as f32);
    let path = builder.to_path();
    let stroke = gsk::Stroke::new(width);
    snapshot.append_stroke(&path, &stroke, color);
}

/// Builds a smoothed curve through `points`, matching Mermaid's basis-curve
/// look for edges (see e.g. the `C` commands merman-render emits for its own
/// SVG edge paths, even for otherwise-straight runs). Each interior point
/// becomes a quadratic Bezier control point with the midpoint to the next
/// point as the curve's end, so the line passes near every layout point
/// without sharp angles at each waypoint.
fn build_smooth_path(points: &[LayoutPoint]) -> gsk::Path {
    let builder = gsk::PathBuilder::new();
    if points.len() < 2 {
        return builder.to_path();
    }
    builder.move_to(points[0].x as f32, points[0].y as f32);
    if points.len() == 2 {
        builder.line_to(points[1].x as f32, points[1].y as f32);
        return builder.to_path();
    }
    for pair in points[1..].windows(2) {
        let cur = &pair[0];
        let next = &pair[1];
        let mid_x = ((cur.x + next.x) / 2.0) as f32;
        let mid_y = ((cur.y + next.y) / 2.0) as f32;
        builder.quad_to(cur.x as f32, cur.y as f32, mid_x, mid_y);
    }
    let last = &points[points.len() - 1];
    let second_last = &points[points.len() - 2];
    builder.quad_to(second_last.x as f32, second_last.y as f32, last.x as f32, last.y as f32);
    builder.to_path()
}

pub fn stroke_smooth_polyline(
    snapshot: &gtk::Snapshot,
    points: &[LayoutPoint],
    color: &gdk::RGBA,
    width: f32,
    dash: Option<&[f32]>,
) {
    if points.len() < 2 {
        return;
    }
    let path = build_smooth_path(points);
    let stroke = gsk::Stroke::new(width);
    if let Some(dash) = dash {
        stroke.set_dash(dash);
    }
    snapshot.append_stroke(&path, &stroke, color);
}

pub fn fill_polygon(snapshot: &gtk::Snapshot, points: &[Pt], color: &gdk::RGBA) {
    if points.len() < 3 {
        return;
    }
    let builder = gsk::PathBuilder::new();
    builder.move_to(points[0].0 as f32, points[0].1 as f32);
    for p in &points[1..] {
        builder.line_to(p.0 as f32, p.1 as f32);
    }
    builder.close();
    let path = builder.to_path();
    snapshot.append_fill(&path, gsk::FillRule::Winding, color);
}

pub fn stroke_polygon(snapshot: &gtk::Snapshot, points: &[Pt], color: &gdk::RGBA, width: f32) {
    if points.len() < 3 {
        return;
    }
    let builder = gsk::PathBuilder::new();
    builder.move_to(points[0].0 as f32, points[0].1 as f32);
    for p in &points[1..] {
        builder.line_to(p.0 as f32, p.1 as f32);
    }
    builder.close();
    let path = builder.to_path();
    let stroke = gsk::Stroke::new(width);
    snapshot.append_stroke(&path, &stroke, color);
}

fn build_layout(
    pango_ctx: &pango::Context,
    text: &str,
    font_family: Option<&str>,
    font_size: f64,
    bold: bool,
) -> pango::Layout {
    let layout = pango::Layout::new(pango_ctx);
    let mut desc = pango::FontDescription::new();
    if let Some(family) = font_family {
        desc.set_family(family);
    }
    desc.set_absolute_size(font_size * f64::from(pango::SCALE));
    if bold {
        desc.set_weight(pango::Weight::Bold);
    }
    layout.set_font_description(Some(&desc));
    layout.set_text(text);
    layout
}

#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    snapshot: &gtk::Snapshot,
    pango_ctx: &pango::Context,
    text: &str,
    x: f64,
    y: f64,
    color: &gdk::RGBA,
    font_family: Option<&str>,
    font_size: f64,
    bold: bool,
    align: HAlign,
) {
    if text.trim().is_empty() {
        return;
    }
    let layout = build_layout(pango_ctx, text, font_family, font_size, bold);
    let (w, h) = layout.pixel_size();

    let draw_x = match align {
        HAlign::Center => x - f64::from(w) / 2.0,
        HAlign::Left => x,
    };
    let draw_y = y - f64::from(h) / 2.0;

    snapshot.save();
    snapshot.translate(&graphene::Point::new(draw_x as f32, draw_y as f32));
    snapshot.append_layout(&layout, color);
    snapshot.restore();
}

/// Draws text centered at `(x, y)` over a filled background rect, matching
/// Mermaid's `.edgeLabel .label rect{fill:...}` styling for relation/edge
/// labels (unlike node or note text, which has no per-label background).
#[allow(clippy::too_many_arguments)]
pub fn draw_text_with_background(
    snapshot: &gtk::Snapshot,
    pango_ctx: &pango::Context,
    text: &str,
    x: f64,
    y: f64,
    text_color: &gdk::RGBA,
    background: &gdk::RGBA,
    font_family: Option<&str>,
    font_size: f64,
) {
    if text.trim().is_empty() {
        return;
    }
    const PAD_X: f64 = 4.0;
    const PAD_Y: f64 = 1.0;

    let layout = build_layout(pango_ctx, text, font_family, font_size, false);
    let (w, h) = layout.pixel_size();
    let (w, h) = (f64::from(w), f64::from(h));

    let bg_rect = graphene::Rect::new(
        (x - w / 2.0 - PAD_X) as f32,
        (y - h / 2.0 - PAD_Y) as f32,
        (w + PAD_X * 2.0) as f32,
        (h + PAD_Y * 2.0) as f32,
    );
    fill_rect(snapshot, &bg_rect, background);

    snapshot.save();
    snapshot.translate(&graphene::Point::new((x - w / 2.0) as f32, (y - h / 2.0) as f32));
    snapshot.append_layout(&layout, text_color);
    snapshot.restore();
}
