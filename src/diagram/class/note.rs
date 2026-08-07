//! `note for X "..."` boxes tethered to a class.

use gtk::graphene;
use gtk::pango;

use merman_core::models::class_diagram::ClassNote;
use merman_render::model::LayoutNode;

use crate::diagram::Theme;
use crate::diagram::draw::{HAlign, draw_text, fill_rect, stroke_rect};

use super::node::node_rect;
use super::scene::ClassDiagramScene;

impl ClassDiagramScene {
    pub(super) fn draw_note_node(
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
        draw_text(
            snapshot,
            pango_ctx,
            &note.text,
            node.x,
            node.y,
            &theme.text,
            self.text_style.font_family.as_deref(),
            self.text_style.font_size,
            false,
            HAlign::Center,
        );
    }
}
