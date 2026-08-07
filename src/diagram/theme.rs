use gtk::gdk;

/// Flat color palette matching Mermaid's default theme values for class
/// diagrams (`ClassDiagramTheme` defaults in `merman_render::svg::parity::theme`:
/// `mainBkg` #ECECFF, `nodeBorder` #9370DB, `clusterBkg` #ffffde, `clusterBorder`
/// #aaaa33, `lineColor`/`titleColor` #333). Custom `classDef` styling and
/// Mermaid "handDrawn" look are not applied yet.
#[derive(Debug, Clone)]
pub struct Theme {
    pub background: gdk::RGBA,
    pub node_fill: gdk::RGBA,
    pub node_stroke: gdk::RGBA,
    pub note_fill: gdk::RGBA,
    pub note_stroke: gdk::RGBA,
    pub cluster_fill: gdk::RGBA,
    pub cluster_stroke: gdk::RGBA,
    pub edge_stroke: gdk::RGBA,
    pub text: gdk::RGBA,
    pub divider: gdk::RGBA,
}

fn rgba(r: u8, g: u8, b: u8) -> gdk::RGBA {
    gdk::RGBA::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: rgba(255, 255, 255),
            node_fill: rgba(236, 236, 255),
            node_stroke: rgba(147, 112, 219),
            note_fill: rgba(255, 245, 173),
            note_stroke: rgba(170, 170, 51),
            cluster_fill: rgba(255, 255, 222),
            cluster_stroke: rgba(170, 170, 51),
            edge_stroke: rgba(51, 51, 51),
            text: rgba(51, 51, 51),
            divider: rgba(147, 112, 219),
        }
    }
}
