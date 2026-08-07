//! Owns the parsed model + computed layout for one class diagram, and
//! dispatches drawing/hit-testing to the per-element-kind modules in this
//! folder (`node`, `note`, `cluster`, `edge`).

use std::collections::{HashMap, HashSet};

use gtk::pango;

use merman_core::MermaidConfig;
use merman_core::models::class_diagram::ClassDiagram;
use merman_render::class::layout_class_diagram_v2_typed_with_config;
use merman_render::model::ClassDiagramV2Layout;
use merman_render::text::VendoredFontMetricsTextMeasurer;
use merman_render::text::TextStyle;

use crate::diagram::{DiagramError, ElementId, Theme};

pub struct ClassDiagramScene {
    pub(super) model: ClassDiagram,
    pub(super) layout: ClassDiagramV2Layout,
    pub(super) measurer: VendoredFontMetricsTextMeasurer,
    pub(super) text_style: TextStyle,
    pub(super) class_padding: f64,
    pub(super) hide_empty_members_box: bool,
    pub(super) relations_by_id: HashMap<String, usize>,
    pub(super) notes_by_id: HashMap<String, usize>,
    /// Dagre's compound-cluster layout adds a synthetic placeholder node for
    /// each namespace cluster (same id, same geometry) purely for sizing
    /// purposes; it shows up in `layout.nodes` too and must be skipped when
    /// drawing/hit-testing individual nodes, or it paints an opaque
    /// rectangle over the namespace's real children.
    pub(super) cluster_ids: HashSet<String>,
}

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
            if super::node::node_contains(node, x, y) {
                return Some(ElementId::Node(node.id.clone()));
            }
        }
        for edge in &self.layout.edges {
            if crate::diagram::geom::polyline_hit(x, y, &edge.points, tolerance) {
                return Some(ElementId::Edge(edge.id.clone()));
            }
        }
        for cluster in &self.layout.clusters {
            if super::cluster::cluster_contains(cluster, x, y) {
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
}
