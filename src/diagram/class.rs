//! Class diagram scene: layout via merman-core/merman-render, drawing via GSK.
//!
//! merman-render's own class diagram renderer (`merman_render::svg::parity::class`)
//! builds an SVG string directly from [`ClassDiagramV2Layout`] geometry. We reuse
//! that same layout (dagre-based box placement, member/method row metrics, edge
//! routing) but draw it ourselves with `gtk::Snapshot`/GSK instead of walking an
//! XML tree, since merman-render has no GSK backend.
//!
//! Compartment (title/members/methods) internal proportions are recomputed here
//! from the semantic model rather than replicated pixel-for-pixel from the SVG
//! renderer's private HTML-label layout code; the outer box always matches
//! [`LayoutNode`] exactly (edges route to it), so this stays visually correct
//! even though internal divider placement is an approximation.

use std::collections::HashMap;

use gtk::gdk;
use gtk::graphene;
use gtk::gsk;
use gtk::pango;
use gtk::prelude::*;

use merman_core::MermaidConfig;
use merman_core::models::class_diagram::{
    ClassDiagram, ClassNode, ClassNote, ClassRelation, ClassRelationTypeConstants,
};
use merman_render::class::layout_class_diagram_v2_typed_with_config;
use merman_render::model::{ClassDiagramV2Layout, LayoutCluster, LayoutEdge, LayoutNode, LayoutPoint};
use merman_render::text::{TextMeasurer, TextStyle, VendoredFontMetricsTextMeasurer};

use super::{DiagramError, ElementId, Theme};

const NODE_PAD_X: f64 = 8.0;
const ROW_MIN_H: f64 = 14.0;
const MARKER_LEN: f64 = 14.0;
const MARKER_HALF_W: f64 = 6.0;
const LABEL_FONT_SCALE: f64 = 0.85;

pub struct ClassDiagramScene {
    model: ClassDiagram,
    layout: ClassDiagramV2Layout,
    measurer: VendoredFontMetricsTextMeasurer,
    text_style: TextStyle,
    class_padding: f64,
    hide_empty_members_box: bool,
    relations_by_id: HashMap<String, usize>,
    notes_by_id: HashMap<String, usize>,
    cluster_ids: std::collections::HashSet<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum MarkerKind {
    None,
    Aggregation,
    Extension,
    Composition,
    Dependency,
    Lollipop,
}

#[derive(Clone, Copy)]
enum HAlign {
    Center,
    Left,
}

type Pt = (f64, f64);

impl ClassDiagramScene {
    pub fn build(model: ClassDiagram, effective_config: &MermaidConfig) -> Result<Self, DiagramError> {
        let measurer = VendoredFontMetricsTextMeasurer::default();
        let layout = layout_class_diagram_v2_typed_with_config(&model, effective_config, &measurer)
            .map_err(|err| DiagramError::Layout(err.to_string()))?;

        let font_family = effective_config
            .get_str("fontFamily")
            .map(str::to_string)
            .or_else(|| Some("sans-serif".to_string()));
        let font_size = effective_config
            .as_value()
            .get("fontSize")
            .and_then(|v| v.as_f64())
            .unwrap_or(16.0);
        let text_style = TextStyle {
            font_family,
            font_size,
            font_weight: None,
        };

        let class_padding = effective_config
            .as_value()
            .get("class")
            .and_then(|v| v.get("padding"))
            .and_then(|v| v.as_f64())
            .unwrap_or(12.0);
        let hide_empty_members_box = effective_config
            .get_bool("class.hideEmptyMembersBox")
            .unwrap_or(false);

        let relations_by_id = model
            .relations
            .iter()
            .enumerate()
            .map(|(i, r)| (r.id.clone(), i))
            .collect();
        let notes_by_id = model
            .notes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect();
        // Dagre's compound-cluster layout adds a synthetic placeholder node for each
        // namespace cluster (same id, same geometry) purely for sizing purposes; it
        // shows up in `layout.nodes` too and must be skipped when drawing/hit-testing
        // individual nodes, or it paints an opaque rectangle over the namespace's
        // real children.
        let cluster_ids = layout.clusters.iter().map(|c| c.id.clone()).collect();

        Ok(Self {
            model,
            layout,
            measurer,
            text_style,
            class_padding,
            hide_empty_members_box,
            relations_by_id,
            notes_by_id,
            cluster_ids,
        })
    }

    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        match &self.layout.bounds {
            Some(b) => (b.min_x, b.min_y, b.max_x, b.max_y),
            None => (0.0, 0.0, 200.0, 100.0),
        }
    }

    pub fn hit_test(&self, x: f64, y: f64, tolerance: f64) -> Option<ElementId> {
        for node in self.layout.nodes.iter().rev() {
            if self.cluster_ids.contains(&node.id) {
                continue;
            }
            let (l, t, r, b) = node_rect(node);
            if x >= l && x <= r && y >= t && y <= b {
                return Some(ElementId::Node(node.id.clone()));
            }
        }
        for edge in &self.layout.edges {
            if polyline_hit(x, y, &edge.points, tolerance) {
                return Some(ElementId::Edge(edge.id.clone()));
            }
        }
        for cluster in &self.layout.clusters {
            let (l, t, r, b) = cluster_rect(cluster);
            if x >= l && x <= r && y >= t && y <= b {
                return Some(ElementId::Cluster(cluster.id.clone()));
            }
        }
        None
    }

    pub fn draw(&self, snapshot: &gtk::Snapshot, pango_ctx: &pango::Context, theme: &Theme) {
        for cluster in &self.layout.clusters {
            self.draw_cluster(snapshot, pango_ctx, theme, cluster);
        }

        for edge in &self.layout.edges {
            match self.relations_by_id.get(edge.id.as_str()) {
                Some(&idx) if !edge.id.starts_with("edgeNote") => {
                    self.draw_relation_edge(snapshot, pango_ctx, theme, edge, &self.model.relations[idx]);
                }
                _ => self.draw_tether_edge(snapshot, theme, edge),
            }
        }

        for node in &self.layout.nodes {
            if self.cluster_ids.contains(&node.id) {
                continue;
            }
            if let Some(class) = self.model.classes.get(node.id.as_str()) {
                self.draw_class_node(snapshot, pango_ctx, theme, node, class);
            } else if let Some(&idx) = self.notes_by_id.get(node.id.as_str()) {
                self.draw_note_node(snapshot, pango_ctx, theme, node, &self.model.notes[idx]);
            } else {
                self.draw_fallback_node(snapshot, pango_ctx, theme, node);
            }
        }
    }

    // -- nodes ---------------------------------------------------------

    fn draw_class_node(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        node: &LayoutNode,
        class: &ClassNode,
    ) {
        let (l, t, r, _b) = node_rect(node);
        let rect = graphene::Rect::new(l as f32, t as f32, node.width as f32, node.height as f32);
        fill_rect(snapshot, &rect, &theme.node_fill);
        stroke_rect(snapshot, &rect, &theme.node_stroke, 1.0, None);

        let padding = self.class_padding;

        let mut title_rows: Vec<String> = Vec::new();
        for annotation in &class.annotations {
            title_rows.push(format!("«{annotation}»"));
        }
        let title_text = if class.type_param.trim().is_empty() {
            class.label.clone()
        } else {
            format!("{} <{}>", class.label, class.type_param)
        };
        title_rows.push(title_text);

        let member_rows: Vec<&str> = class.members.iter().map(|m| m.display_text.as_str()).collect();
        let method_rows: Vec<&str> = class.methods.iter().map(|m| m.display_text.as_str()).collect();

        let members_present = !member_rows.is_empty() || !self.hide_empty_members_box;
        let methods_present = !method_rows.is_empty() || !self.hide_empty_members_box;

        let title_texts: Vec<&str> = title_rows.iter().map(String::as_str).collect();
        let (mut title_heights, mut title_h) = self.measure_block(&title_texts, padding);
        let (mut member_heights, mut members_h) = if members_present {
            self.measure_block(&member_rows, padding)
        } else {
            (Vec::new(), 0.0)
        };
        let (mut method_heights, methods_h) = if methods_present {
            self.measure_block(&method_rows, padding)
        } else {
            (Vec::new(), 0.0)
        };

        let total = title_h + members_h + methods_h;
        let fit_scale = if total > 1e-6 { node.height / total } else { 1.0 };
        for h in title_heights.iter_mut().chain(member_heights.iter_mut()).chain(method_heights.iter_mut()) {
            *h *= fit_scale;
        }
        title_h *= fit_scale;
        members_h *= fit_scale;
        let _ = methods_h; // final block height isn't needed past this point (nothing follows it)

        self.draw_row_block(
            snapshot,
            pango_ctx,
            theme,
            &title_texts,
            &title_heights,
            t,
            padding * fit_scale,
            node.x,
            HAlign::Center,
            true,
        );

        let mut cursor = t + title_h;
        if members_present {
            stroke_hline(snapshot, l, r, cursor, &theme.divider, 1.0);
            self.draw_row_block(
                snapshot,
                pango_ctx,
                theme,
                &member_rows,
                &member_heights,
                cursor,
                padding * fit_scale,
                l + NODE_PAD_X,
                HAlign::Left,
                false,
            );
            cursor += members_h;
        }
        if methods_present {
            stroke_hline(snapshot, l, r, cursor, &theme.divider, 1.0);
            self.draw_row_block(
                snapshot,
                pango_ctx,
                theme,
                &method_rows,
                &method_heights,
                cursor,
                padding * fit_scale,
                l + NODE_PAD_X,
                HAlign::Left,
                false,
            );
        }
    }

    fn draw_note_node(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        node: &LayoutNode,
        note: &ClassNote,
    ) {
        let (l, t, _r, _b) = node_rect(node);
        let rect = graphene::Rect::new(l as f32, t as f32, node.width as f32, node.height as f32);
        fill_rect(snapshot, &rect, &theme.note_fill);
        stroke_rect(snapshot, &rect, &theme.note_stroke, 1.0, None);
        self.draw_text(
            snapshot,
            pango_ctx,
            &note.text,
            node.x,
            node.y,
            &theme.text,
            false,
            HAlign::Center,
        );
    }

    fn draw_fallback_node(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        node: &LayoutNode,
    ) {
        let (l, t, _r, _b) = node_rect(node);
        let rect = graphene::Rect::new(l as f32, t as f32, node.width as f32, node.height as f32);
        fill_rect(snapshot, &rect, &theme.node_fill);
        stroke_rect(snapshot, &rect, &theme.node_stroke, 1.0, None);
        self.draw_text(
            snapshot,
            pango_ctx,
            &node.id,
            node.x,
            node.y,
            &theme.text,
            false,
            HAlign::Center,
        );
    }

    fn draw_cluster(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        cluster: &LayoutCluster,
    ) {
        let (l, t, r, b) = cluster_rect(cluster);
        let rect = graphene::Rect::new(l as f32, t as f32, cluster.width as f32, cluster.height as f32);
        fill_rect(snapshot, &rect, &theme.cluster_fill);
        stroke_rect(snapshot, &rect, &theme.cluster_stroke, 1.0, Some(&[6.0, 4.0]));

        if !cluster.title.trim().is_empty() {
            self.draw_text(
                snapshot,
                pango_ctx,
                &cluster.title,
                cluster.title_label.x,
                cluster.title_label.y,
                &theme.text,
                true,
                HAlign::Center,
            );
        }
        let _ = r;
        let _ = b;
    }

    // -- compartment rows -----------------------------------------------

    /// Measures a compartment's row heights and total block height (including
    /// top/bottom padding), using the real Mermaid-parity text measurer so
    /// stacking order roughly tracks Mermaid's own line heights.
    fn measure_row_height(&self, text: &str) -> f64 {
        if text.trim().is_empty() {
            self.text_style.font_size * 1.3
        } else {
            self.measurer
                .measure(text, &self.text_style)
                .height
                .max(ROW_MIN_H)
        }
    }

    fn measure_block(&self, rows: &[&str], padding: f64) -> (Vec<f64>, f64) {
        let heights: Vec<f64> = if rows.is_empty() {
            vec![self.text_style.font_size * 1.3]
        } else {
            rows.iter().map(|r| self.measure_row_height(r)).collect()
        };
        let content: f64 = heights.iter().sum();
        (heights, content + padding * 2.0)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_row_block(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        rows: &[&str],
        row_heights: &[f64],
        top: f64,
        padding: f64,
        anchor_x: f64,
        align: HAlign,
        bold_last: bool,
    ) {
        if rows.is_empty() {
            return;
        }
        let mut cursor = top + padding;
        for (i, text) in rows.iter().enumerate() {
            let h = row_heights.get(i).copied().unwrap_or(self.text_style.font_size);
            let cy = cursor + h / 2.0;
            let bold = bold_last && i == rows.len() - 1;
            self.draw_text(snapshot, pango_ctx, text, anchor_x, cy, &theme.text, bold, align);
            cursor += h;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_text(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        text: &str,
        x: f64,
        y: f64,
        color: &gdk::RGBA,
        bold: bool,
        align: HAlign,
    ) {
        if text.trim().is_empty() {
            return;
        }
        let layout = pango::Layout::new(pango_ctx);
        let mut desc = pango::FontDescription::new();
        if let Some(family) = &self.text_style.font_family {
            desc.set_family(family);
        }
        desc.set_absolute_size(self.text_style.font_size * f64::from(pango::SCALE));
        if bold {
            desc.set_weight(pango::Weight::Bold);
        }
        layout.set_font_description(Some(&desc));
        layout.set_text(text);
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

    // -- edges ------------------------------------------------------------

    fn draw_tether_edge(&self, snapshot: &gtk::Snapshot, theme: &Theme, edge: &LayoutEdge) {
        stroke_polyline(snapshot, &edge.points, &theme.note_stroke, 1.0, Some(&[4.0, 3.0]));
    }

    fn draw_relation_edge(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        edge: &LayoutEdge,
        relation: &ClassRelation,
    ) {
        let dashed = relation.relation.line_type == self.model.constants.line_type.dotted_line;
        let dash: Option<&[f32]> = if dashed { Some(&[5.0, 4.0]) } else { None };
        stroke_polyline(snapshot, &edge.points, &theme.edge_stroke, 1.5, dash);

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
            && !relation.title.trim().is_empty() {
                self.draw_small_text(snapshot, pango_ctx, &relation.title, label.x, label.y, theme);
            }
        if relation.relation_title_1 != "none"
            && let Some(label) = edge.start_label_left.as_ref().or(edge.start_label_right.as_ref()) {
                self.draw_small_text(
                    snapshot,
                    pango_ctx,
                    &relation.relation_title_1,
                    label.x,
                    label.y,
                    theme,
                );
            }
        if relation.relation_title_2 != "none"
            && let Some(label) = edge.end_label_left.as_ref().or(edge.end_label_right.as_ref()) {
                self.draw_small_text(
                    snapshot,
                    pango_ctx,
                    &relation.relation_title_2,
                    label.x,
                    label.y,
                    theme,
                );
            }
    }

    fn draw_small_text(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        text: &str,
        x: f64,
        y: f64,
        theme: &Theme,
    ) {
        let layout = pango::Layout::new(pango_ctx);
        let mut desc = pango::FontDescription::new();
        if let Some(family) = &self.text_style.font_family {
            desc.set_family(family);
        }
        desc.set_absolute_size(self.text_style.font_size * LABEL_FONT_SCALE * f64::from(pango::SCALE));
        layout.set_font_description(Some(&desc));
        layout.set_text(text);
        let (w, h) = layout.pixel_size();

        snapshot.save();
        snapshot.translate(&graphene::Point::new(
            (x - f64::from(w) / 2.0) as f32,
            (y - f64::from(h) / 2.0) as f32,
        ));
        snapshot.append_layout(&layout, &theme.text);
        snapshot.restore();
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
                let a = add(tip, add(scale(dir, MARKER_LEN), scale(perp, MARKER_HALF_W)));
                let b = add(tip, sub(scale(dir, MARKER_LEN), scale(perp, MARKER_HALF_W)));
                let builder = gsk::PathBuilder::new();
                builder.move_to(a.0 as f32, a.1 as f32);
                builder.line_to(tip.0 as f32, tip.1 as f32);
                builder.line_to(b.0 as f32, b.1 as f32);
                let path = builder.to_path();
                let stroke = gsk::Stroke::new(1.0);
                snapshot.append_stroke(&path, &stroke, &theme.edge_stroke);
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

fn node_rect(node: &LayoutNode) -> (f64, f64, f64, f64) {
    (
        node.x - node.width / 2.0,
        node.y - node.height / 2.0,
        node.x + node.width / 2.0,
        node.y + node.height / 2.0,
    )
}

fn cluster_rect(cluster: &LayoutCluster) -> (f64, f64, f64, f64) {
    (
        cluster.x - cluster.width / 2.0,
        cluster.y - cluster.height / 2.0,
        cluster.x + cluster.width / 2.0,
        cluster.y + cluster.height / 2.0,
    )
}

fn polyline_hit(x: f64, y: f64, points: &[LayoutPoint], tolerance: f64) -> bool {
    points
        .windows(2)
        .any(|seg| point_segment_distance((x, y), pt(&seg[0]), pt(&seg[1])) <= tolerance)
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

fn pt(p: &LayoutPoint) -> Pt {
    (p.x, p.y)
}
fn add(a: Pt, b: Pt) -> Pt {
    (a.0 + b.0, a.1 + b.1)
}
fn sub(a: Pt, b: Pt) -> Pt {
    (a.0 - b.0, a.1 - b.1)
}
fn scale(a: Pt, s: f64) -> Pt {
    (a.0 * s, a.1 * s)
}
fn dist(a: Pt, b: Pt) -> f64 {
    let d = sub(a, b);
    (d.0 * d.0 + d.1 * d.1).sqrt()
}
fn normalize(v: Pt) -> Pt {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len <= 1e-9 { (0.0, 1.0) } else { (v.0 / len, v.1 / len) }
}

fn fill_rect(snapshot: &gtk::Snapshot, rect: &graphene::Rect, color: &gdk::RGBA) {
    snapshot.append_color(color, rect);
}

fn stroke_rect(
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

fn stroke_hline(snapshot: &gtk::Snapshot, x0: f64, x1: f64, y: f64, color: &gdk::RGBA, width: f32) {
    let builder = gsk::PathBuilder::new();
    builder.move_to(x0 as f32, y as f32);
    builder.line_to(x1 as f32, y as f32);
    let path = builder.to_path();
    let stroke = gsk::Stroke::new(width);
    snapshot.append_stroke(&path, &stroke, color);
}

fn stroke_polyline(
    snapshot: &gtk::Snapshot,
    points: &[LayoutPoint],
    color: &gdk::RGBA,
    width: f32,
    dash: Option<&[f32]>,
) {
    if points.len() < 2 {
        return;
    }
    let builder = gsk::PathBuilder::new();
    builder.move_to(points[0].x as f32, points[0].y as f32);
    for p in &points[1..] {
        builder.line_to(p.x as f32, p.y as f32);
    }
    let path = builder.to_path();
    let stroke = gsk::Stroke::new(width);
    if let Some(dash) = dash {
        stroke.set_dash(dash);
    }
    snapshot.append_stroke(&path, &stroke, color);
}

fn fill_polygon(snapshot: &gtk::Snapshot, points: &[Pt], color: &gdk::RGBA) {
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

fn stroke_polygon(snapshot: &gtk::Snapshot, points: &[Pt], color: &gdk::RGBA, width: f32) {
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
