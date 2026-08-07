//! Relation edges between classes: the routed curve, its UML arrow markers
//! (aggregation, composition, extension/realization, dependency, lollipop),
//! and the relation title / multiplicity labels.
//!
//! `note for X` tether edges (id prefixed `edgeNote`) have no semantic
//! relation behind them and are drawn as a plain dashed curve with no
//! markers or labels.

use gtk::prelude::*;
use gtk::{graphene, gsk, pango};

use merman_core::models::class_diagram::{ClassRelation, ClassRelationTypeConstants};
use merman_render::model::LayoutEdge;

use crate::diagram::Theme;
use crate::diagram::draw::{draw_text_with_background, fill_polygon, stroke_polygon, stroke_smooth_polyline};
use crate::diagram::geom::{Pt, add, normalize, pt, scale, sub};

use super::scene::ClassDiagramScene;

const MARKER_LEN: f64 = 14.0;
const MARKER_HALF_W: f64 = 6.0;

/// Matches Mermaid's `.edgeTerminals{font-size:11px}` rule for multiplicity/
/// role labels near relation endpoints (e.g. "1", "*"). The main relation
/// title ("uses", "depends on", ...) has no such override and renders at the
/// diagram's normal text size.
const EDGE_TERMINAL_FONT_SIZE: f64 = 11.0;

#[derive(Clone, Copy, PartialEq)]
enum MarkerKind {
    None,
    Aggregation,
    Extension,
    Composition,
    Dependency,
    Lollipop,
}

fn marker_kind(value: i32, constants: &ClassRelationTypeConstants) -> MarkerKind {
    if value == constants.aggregation {
        MarkerKind::Aggregation
    } else if value == constants.extension {
        MarkerKind::Extension
    } else if value == constants.composition {
        MarkerKind::Composition
    } else if value == constants.dependency {
        MarkerKind::Dependency
    } else if value == constants.lollipop {
        MarkerKind::Lollipop
    } else {
        MarkerKind::None
    }
}

impl ClassDiagramScene {
    pub(super) fn draw_tether_edge(&self, snapshot: &gtk::Snapshot, theme: &Theme, edge: &LayoutEdge) {
        stroke_smooth_polyline(snapshot, &edge.points, &theme.note_stroke, 1.0, Some(&[4.0, 3.0]));
    }

    pub(super) fn draw_relation_edge(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        edge: &LayoutEdge,
        relation: &ClassRelation,
    ) {
        let dashed = relation.relation.line_type == self.model.constants.line_type.dotted_line;
        let dash: Option<&[f32]> = if dashed { Some(&[2.0, 3.0]) } else { None };
        stroke_smooth_polyline(snapshot, &edge.points, &theme.edge_stroke, 1.0, dash);

        if edge.points.len() >= 2 {
            let constants = &self.model.constants.relation_type;
            let start = pt(&edge.points[0]);
            let next = pt(&edge.points[1]);
            let start_dir = normalize(sub(next, start));
            self.draw_marker(
                snapshot,
                theme,
                start,
                start_dir,
                marker_kind(relation.relation.type1, constants),
            );

            let end = pt(&edge.points[edge.points.len() - 1]);
            let before_end = pt(&edge.points[edge.points.len() - 2]);
            let end_dir = normalize(sub(before_end, end));
            self.draw_marker(
                snapshot,
                theme,
                end,
                end_dir,
                marker_kind(relation.relation.type2, constants),
            );
        }

        if let Some(label) = &edge.label
            && !relation.title.trim().is_empty()
        {
            self.draw_edge_text(
                snapshot,
                pango_ctx,
                &relation.title,
                label.x,
                label.y,
                theme,
                self.text_style.font_size,
            );
        }
        if relation.relation_title_1 != "none"
            && let Some(label) = edge.start_label_left.as_ref().or(edge.start_label_right.as_ref())
        {
            self.draw_edge_text(
                snapshot,
                pango_ctx,
                &relation.relation_title_1,
                label.x,
                label.y,
                theme,
                EDGE_TERMINAL_FONT_SIZE,
            );
        }
        if relation.relation_title_2 != "none"
            && let Some(label) = edge.end_label_left.as_ref().or(edge.end_label_right.as_ref())
        {
            self.draw_edge_text(
                snapshot,
                pango_ctx,
                &relation.relation_title_2,
                label.x,
                label.y,
                theme,
                EDGE_TERMINAL_FONT_SIZE,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_edge_text(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        text: &str,
        x: f64,
        y: f64,
        theme: &Theme,
        font_size: f64,
    ) {
        draw_text_with_background(
            snapshot,
            pango_ctx,
            text,
            x,
            y,
            &theme.text,
            &theme.node_fill,
            self.text_style.font_family.as_deref(),
            font_size,
        );
    }

    fn draw_marker(&self, snapshot: &gtk::Snapshot, theme: &Theme, tip: Pt, dir: Pt, kind: MarkerKind) {
        if kind == MarkerKind::None {
            return;
        }
        let perp = (-dir.1, dir.0);

        match kind {
            MarkerKind::Extension => {
                let base_l = add(tip, add(scale(dir, MARKER_LEN), scale(perp, MARKER_HALF_W)));
                let base_r = add(tip, sub(scale(dir, MARKER_LEN), scale(perp, MARKER_HALF_W)));
                fill_polygon(snapshot, &[tip, base_l, base_r], &theme.background);
                stroke_polygon(snapshot, &[tip, base_l, base_r], &theme.node_stroke, 1.0);
            }
            MarkerKind::Aggregation | MarkerKind::Composition => {
                let mid = add(tip, scale(dir, MARKER_LEN / 2.0));
                let far = add(tip, scale(dir, MARKER_LEN));
                let left = add(mid, scale(perp, MARKER_HALF_W));
                let right = sub(mid, scale(perp, MARKER_HALF_W));
                let points = [tip, left, far, right];
                let fill = if kind == MarkerKind::Composition {
                    &theme.node_stroke
                } else {
                    &theme.background
                };
                fill_polygon(snapshot, &points, fill);
                stroke_polygon(snapshot, &points, &theme.node_stroke, 1.0);
            }
            MarkerKind::Dependency => {
                // A solid filled arrowhead (Mermaid's `.dependency` marker is
                // filled, unlike `.extension`/`.aggregation` which are hollow).
                let a = add(tip, add(scale(dir, MARKER_LEN), scale(perp, MARKER_HALF_W)));
                let b = add(tip, sub(scale(dir, MARKER_LEN), scale(perp, MARKER_HALF_W)));
                fill_polygon(snapshot, &[tip, a, b], &theme.edge_stroke);
            }
            MarkerKind::Lollipop => {
                let center = add(tip, scale(dir, MARKER_HALF_W));
                let builder = gsk::PathBuilder::new();
                builder.add_circle(&graphene::Point::new(center.0 as f32, center.1 as f32), MARKER_HALF_W as f32);
                let path = builder.to_path();
                snapshot.append_fill(&path, gsk::FillRule::Winding, &theme.background);
                let stroke = gsk::Stroke::new(1.0);
                snapshot.append_stroke(&path, &stroke, &theme.node_stroke);
            }
            MarkerKind::None => {}
        }
    }
}
