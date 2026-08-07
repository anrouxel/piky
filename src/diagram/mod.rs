//! Mermaid source -> semantic model + layout, independent of any rendering backend.
//!
//! merman-render only emits SVG. This module reuses merman-core (parsing) and
//! merman-render's layout algorithms (node/edge geometry), then hands the typed
//! result to `crate::canvas` for GSK drawing and hit-testing. Adding a new
//! Mermaid diagram type means adding a new variant here plus a `draw`/`hit_test`
//! implementation under this module, following the `class` module as a template.

pub mod class;
pub mod theme;

use merman_core::diagram::RenderSemanticModel;
use merman_core::{Engine, ParseOptions};

pub use theme::Theme;

/// Stable identifier for a diagram element, used for click logging and (later) selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementId {
    Node(String),
    Edge(String),
    Cluster(String),
}

#[derive(Debug, thiserror::Error)]
pub enum DiagramError {
    #[error("no Mermaid diagram detected")]
    NoDiagram,
    #[error("failed to parse diagram: {0}")]
    Parse(#[from] merman_core::Error),
    #[error("failed to lay out diagram: {0}")]
    Layout(String),
    #[error("diagram type '{0}' is not implemented yet")]
    Unsupported(String),
}

/// A parsed and laid-out diagram, ready to be drawn on the canvas.
///
/// Only `classDiagram` is implemented today; other Mermaid diagram types are
/// reported as [`DiagramError::Unsupported`] until a matching module is added
/// here (see the module docs for the pattern to follow).
pub enum Diagram {
    Class(class::ClassDiagramScene),
}

impl Diagram {
    pub fn parse(source: &str) -> Result<Self, DiagramError> {
        let engine = Engine::new();
        let parsed = engine
            .parse_diagram_for_render_model_sync(source, ParseOptions::default())?
            .ok_or(DiagramError::NoDiagram)?;

        match parsed.model {
            RenderSemanticModel::Class(model) => {
                let scene = class::ClassDiagramScene::build(model, &parsed.meta.effective_config)?;
                Ok(Diagram::Class(scene))
            }
            other => Err(DiagramError::Unsupported(diagram_kind_name(&other))),
        }
    }

    /// Bounding box of the diagram content, in diagram-space units.
    pub fn bounds(&self) -> (f64, f64, f64, f64) {
        match self {
            Diagram::Class(scene) => scene.bounds(),
        }
    }

    pub fn draw(&self, snapshot: &gtk::Snapshot, pango_ctx: &gtk::pango::Context, theme: &Theme) {
        match self {
            Diagram::Class(scene) => scene.draw(snapshot, pango_ctx, theme),
        }
    }

    /// Hit-tests a point in diagram-space coordinates. `tolerance` (also
    /// diagram-space units) controls how close a click must be to an edge's
    /// polyline to count as a hit; callers should scale it by `1.0 / zoom` so
    /// it corresponds to a constant on-screen distance.
    pub fn hit_test(&self, x: f64, y: f64, tolerance: f64) -> Option<ElementId> {
        match self {
            Diagram::Class(scene) => scene.hit_test(x, y, tolerance),
        }
    }
}

fn diagram_kind_name(model: &RenderSemanticModel) -> String {
    match model {
        RenderSemanticModel::Json(_) => "json",
        RenderSemanticModel::Mindmap(_) => "mindmap",
        RenderSemanticModel::State(_) => "state",
        RenderSemanticModel::Sequence(_) => "sequence",
        RenderSemanticModel::Flowchart(_) => "flowchart",
        RenderSemanticModel::Architecture(_) => "architecture",
        RenderSemanticModel::Class(_) => "class",
        RenderSemanticModel::C4(_) => "c4",
        RenderSemanticModel::Kanban(_) => "kanban",
        RenderSemanticModel::Gantt(_) => "gantt",
        RenderSemanticModel::Pie(_) => "pie",
        RenderSemanticModel::Packet(_) => "packet",
        RenderSemanticModel::Timeline(_) => "timeline",
        RenderSemanticModel::Journey(_) => "journey",
        RenderSemanticModel::Requirement(_) => "requirement",
        RenderSemanticModel::Sankey(_) => "sankey",
        RenderSemanticModel::Radar(_) => "radar",
        RenderSemanticModel::Info(_) => "info",
        RenderSemanticModel::Treemap(_) => "treemap",
        RenderSemanticModel::Block(_) => "block",
        RenderSemanticModel::Er(_) => "er",
        RenderSemanticModel::QuadrantChart(_) => "quadrantChart",
        RenderSemanticModel::XyChart(_) => "xychart",
        RenderSemanticModel::GitGraph(_) => "gitGraph",
        RenderSemanticModel::TreeView(_) => "treeView",
        _ => "unknown",
    }
    .to_string()
}
