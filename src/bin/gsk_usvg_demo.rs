use gtk::prelude::*;
use gtk::{gdk, graphene, gsk};

use merman_core::{Engine, ParseOptions};
use merman_render::svg::{SvgRenderOptions, render_layouted_svg, resvg_safe_svg};
use merman_render::{LayoutOptions, layout_parsed};

const DEMO_DIAGRAM: &str = r#"
flowchart TD
    A[Start] --> B{Is it working?}
    B -- Yes --> C[Great]
    B -- No --> D[Debug]
    D --> B
"#;

fn diagram_to_svg(source: &str) -> Result<String, Box<dyn std::error::Error>> {
    let engine = Engine::new();

    let parsed = engine
        .parse_diagram_sync(source, ParseOptions::lenient())?
        .ok_or("le diagramme n'a pas pu être parsé")?;

    // headless_svg_defaults() utilise un measurer basé sur des métriques de
    // police vendored (plus proche de ce que produit mermaid.js) plutôt que
    // le measurer déterministe utilisé par log_flowchart_layout().
    let layout_options = LayoutOptions::headless_svg_defaults();
    let layouted = layout_parsed(&parsed, &layout_options)?;

    let svg = render_layouted_svg(
        &layouted,
        layout_options.text_measurer.as_ref(),
        &SvgRenderOptions::default(),
    )?;

    // Le SVG de merman utilise des labels <foreignObject> (HTML), que usvg
    // ignore silencieusement. resvg_safe_svg() les remplace par du <text>
    // natif, exploitable par usvg/resvg.
    Ok(resvg_safe_svg(&svg))
}

fn map_point(mut point: tiny_skia_path::Point, transform: &usvg::Transform) -> (f32, f32) {
    transform.map_point(&mut point);
    (point.x, point.y)
}

fn build_gsk_path(data: &tiny_skia_path::Path, transform: &usvg::Transform) -> gsk::Path {
    let builder = gsk::PathBuilder::new();

    for segment in data.segments() {
        match segment {
            tiny_skia_path::PathSegment::MoveTo(p) => {
                let (x, y) = map_point(p, transform);
                builder.move_to(x, y);
            }
            tiny_skia_path::PathSegment::LineTo(p) => {
                let (x, y) = map_point(p, transform);
                builder.line_to(x, y);
            }
            tiny_skia_path::PathSegment::QuadTo(c, p) => {
                let (cx, cy) = map_point(c, transform);
                let (x, y) = map_point(p, transform);
                builder.quad_to(cx, cy, x, y);
            }
            tiny_skia_path::PathSegment::CubicTo(c1, c2, p) => {
                let (x1, y1) = map_point(c1, transform);
                let (x2, y2) = map_point(c2, transform);
                let (x, y) = map_point(p, transform);
                builder.cubic_to(x1, y1, x2, y2, x, y);
            }
            tiny_skia_path::PathSegment::Close => builder.close(),
        }
    }

    builder.to_path()
}

fn usvg_color_to_rgba(color: usvg::Color, opacity: f32) -> gdk::RGBA {
    gdk::RGBA::new(
        color.red as f32 / 255.0,
        color.green as f32 / 255.0,
        color.blue as f32 / 255.0,
        opacity,
    )
}

fn draw_usvg_path(snapshot: &gtk::Snapshot, path: &usvg::Path) {
    if !path.is_visible() {
        return;
    }

    let transform = path.abs_transform();
    let gsk_path = build_gsk_path(path.data(), &transform);

    if let Some(fill) = path.fill()
        && let usvg::Paint::Color(color) = fill.paint()
    {
        let fill_rule = match fill.rule() {
            usvg::FillRule::NonZero => gsk::FillRule::Winding,
            usvg::FillRule::EvenOdd => gsk::FillRule::EvenOdd,
        };
        snapshot.append_fill(
            &gsk_path,
            fill_rule,
            &usvg_color_to_rgba(*color, fill.opacity().get()),
        );
    }

    if let Some(stroke) = path.stroke()
        && let usvg::Paint::Color(color) = stroke.paint()
    {
        let gsk_stroke = gsk::Stroke::new(stroke.width().get());
        snapshot.append_stroke(
            &gsk_path,
            &gsk_stroke,
            &usvg_color_to_rgba(*color, stroke.opacity().get()),
        );
    }
}

fn draw_usvg_group(snapshot: &gtk::Snapshot, group: &usvg::Group) {
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => draw_usvg_group(snapshot, child),
            usvg::Node::Path(path) => draw_usvg_path(snapshot, path),
            // Le texte est déjà converti en contours de glyphes (des Path) par usvg.
            usvg::Node::Text(text) => draw_usvg_group(snapshot, text.flattened()),
            usvg::Node::Image(_) => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    gtk::init()?;

    let svg = diagram_to_svg(DEMO_DIAGRAM)?;

    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let usvg_options = usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(&svg, &usvg_options)?;
    let size = tree.size();

    let snapshot = gtk::Snapshot::new();
    snapshot.append_color(
        &gdk::RGBA::new(1.0, 1.0, 1.0, 1.0),
        &graphene::Rect::new(0.0, 0.0, size.width(), size.height()),
    );
    draw_usvg_group(&snapshot, tree.root());

    let node = snapshot.to_node().ok_or("snapshot vide")?;

    let surface = cairo::ImageSurface::create(
        cairo::Format::ARgb32,
        size.width().ceil() as i32,
        size.height().ceil() as i32,
    )?;
    let cr = cairo::Context::new(&surface)?;
    node.draw(&cr);

    let out_path = std::env::temp_dir().join("gsk_usvg_demo.png");
    let mut file = std::fs::File::create(&out_path)?;
    surface.write_to_png(&mut file)?;

    println!(
        "Rendu écrit dans {} ({}x{})",
        out_path.display(),
        size.width(),
        size.height()
    );

    Ok(())
}
