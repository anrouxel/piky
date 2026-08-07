//! Class node boxes: title/annotations, members compartment, methods
//! compartment, and the plain-rect fallback for anything unrecognized.

use gtk::graphene;
use gtk::pango;

use merman_core::models::class_diagram::ClassNode;
use merman_render::model::LayoutNode;
use merman_render::text::TextMeasurer;

use crate::diagram::Theme;
use crate::diagram::draw::{HAlign, draw_text, fill_rect, stroke_hline, stroke_rect};

use super::scene::ClassDiagramScene;

const NODE_PAD_X: f64 = 8.0;
const ROW_MIN_H: f64 = 14.0;

pub(super) fn node_rect(node: &LayoutNode) -> (f64, f64, f64, f64) {
    (
        node.x - node.width / 2.0,
        node.y - node.height / 2.0,
        node.x + node.width / 2.0,
        node.y + node.height / 2.0,
    )
}

pub(super) fn node_contains(node: &LayoutNode, x: f64, y: f64) -> bool {
    let (l, t, r, b) = node_rect(node);
    x >= l && x <= r && y >= t && y <= b
}

impl ClassDiagramScene {
    pub(super) fn draw_class_node(
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

    pub(super) fn draw_fallback_node(
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
        draw_text(
            snapshot,
            pango_ctx,
            &node.id,
            node.x,
            node.y,
            &theme.text,
            self.text_style.font_family.as_deref(),
            self.text_style.font_size,
            false,
            HAlign::Center,
        );
    }

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
            draw_text(
                snapshot,
                pango_ctx,
                text,
                anchor_x,
                cy,
                &theme.text,
                self.text_style.font_family.as_deref(),
                self.text_style.font_size,
                bold,
                align,
            );
            cursor += h;
        }
    }
}
