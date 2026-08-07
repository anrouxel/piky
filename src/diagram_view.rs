use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk};

use merman_core::{Engine, ParseOptions};
use merman_render::environment::RenderEnvironment;
use merman_render::{LayoutOptions, family};

/// Parse un texte mermaid et renvoie la géométrie calculée (positions,
/// tailles, couleurs, tracés) sans jamais passer par le rendu SVG de merman.
pub fn layout_flowchart_json(source: &str) -> anyhow::Result<serde_json::Value> {
    let engine = Engine::new();

    // NOTE: si `ParseOptions::default()` ne compile pas, vérifiez la vraie
    // définition de `ParseOptions` (cargo doc -p merman-core --open).
    let parsed = engine
        .parse_diagram_for_render_model_sync(source, ParseOptions::default())?
        .ok_or_else(|| anyhow::anyhow!("le diagramme n'a pas pu être parsé"))?;

    let env = RenderEnvironment::deterministic();
    let session = env.begin_session()?;

    let artifact = family::prepare(parsed, &LayoutOptions::default(), session)?;

    Ok(artifact.layout_json()?)
}

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct DiagramView {
        pub layout: RefCell<Option<serde_json::Value>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DiagramView {
        const NAME: &'static str = "PikyDiagramView";
        type Type = super::DiagramView;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for DiagramView {}

    impl WidgetImpl for DiagramView {
        fn measure(&self, _orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            (600, 600, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;

            snapshot.append_color(
                &gdk::RGBA::new(1.0, 1.0, 1.0, 1.0),
                &graphene::Rect::new(0.0, 0.0, width.max(1.0), height.max(1.0)),
            );

            let Some(layout) = self.layout.borrow().clone() else {
                return;
            };

            // ATTENTION: adaptez ces clés à la structure JSON réelle
            // renvoyée par layout_json(). Décommentez le `debug!` dans
            // `PikyApplicationWindow::setup_widgets` pour l'inspecter.
            if let Some(nodes) = layout.get("nodes").and_then(|v| v.as_array()) {
                for node in nodes {
                    let x = node.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = node.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = node.get("width").and_then(|v| v.as_f64()).unwrap_or(80.0) as f32;
                    let h = node.get("height").and_then(|v| v.as_f64()).unwrap_or(40.0) as f32;
                    let fill = node.get("fill").and_then(|v| v.as_str()).unwrap_or("#ECECFF");
                    let stroke = node.get("stroke").and_then(|v| v.as_str()).unwrap_or("#9370DB");
                    let label = node.get("label").and_then(|v| v.as_str()).unwrap_or("");

                    let rect = graphene::Rect::new(x, y, w, h);
                    let corner = graphene::Size::new(6.0, 6.0);
                    let rounded = gsk::RoundedRect::new(rect, corner, corner, corner, corner);

                    let fill_color = gdk::RGBA::parse(fill).unwrap_or(gdk::RGBA::WHITE);
                    snapshot.push_rounded_clip(&rounded);
                    snapshot.append_color(&fill_color, &rect);
                    snapshot.pop();

                    let stroke_color = gdk::RGBA::parse(stroke).unwrap_or(gdk::RGBA::BLACK);
                    snapshot.append_border(&rounded, &[1.5; 4], &[stroke_color; 4]);

                    if !label.is_empty() {
                        let pango_layout = widget.create_pango_layout(Some(label));
                        snapshot.save();
                        snapshot.translate(&graphene::Point::new(x + 8.0, y + h / 2.0 - 8.0));
                        snapshot.append_layout(&pango_layout, &gdk::RGBA::new(0.1, 0.1, 0.1, 1.0));
                        snapshot.restore();
                    }
                }
            }

            if let Some(edges) = layout.get("edges").and_then(|v| v.as_array()) {
                for edge in edges {
                    if let Some(points) = edge.get("path").and_then(|v| v.as_array()) {
                        if points.is_empty() {
                            continue;
                        }
                        let cr = snapshot.append_cairo(&graphene::Rect::new(
                            0.0,
                            0.0,
                            width.max(1.0),
                            height.max(1.0),
                        ));
                        cr.set_source_rgb(0.4, 0.4, 0.4);
                        cr.set_line_width(1.5);
                        for (i, p) in points.iter().enumerate() {
                            let px = p.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            let py = p.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                            if i == 0 {
                                cr.move_to(px, py);
                            } else {
                                cr.line_to(px, py);
                            }
                        }
                        let _ = cr.stroke();
                    }
                }
            }
        }
    }
}

glib::wrapper! {
    pub struct DiagramView(ObjectSubclass<imp::DiagramView>) @extends gtk::Widget;
}

impl DiagramView {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_layout_json(&self, value: serde_json::Value) {
        *self.imp().layout.borrow_mut() = Some(value);
        self.queue_draw();
    }
}

impl Default for DiagramView {
    fn default() -> Self {
        Self::new()
    }
}
