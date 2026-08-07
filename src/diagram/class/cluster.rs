//! Namespace clusters: a labeled bounding box around a group of classes.

use gtk::graphene;
use gtk::pango;

use merman_render::model::LayoutCluster;

use crate::diagram::Theme;
use crate::diagram::draw::{HAlign, draw_text, fill_rect, stroke_rect};

use super::scene::ClassDiagramScene;

pub(super) fn cluster_rect(cluster: &LayoutCluster) -> (f64, f64, f64, f64) {
    (
        cluster.x - cluster.width / 2.0,
        cluster.y - cluster.height / 2.0,
        cluster.x + cluster.width / 2.0,
        cluster.y + cluster.height / 2.0,
    )
}

pub(super) fn cluster_contains(cluster: &LayoutCluster, x: f64, y: f64) -> bool {
    let (l, t, r, b) = cluster_rect(cluster);
    x >= l && x <= r && y >= t && y <= b
}

impl ClassDiagramScene {
    pub(super) fn draw_cluster(
        &self,
        snapshot: &gtk::Snapshot,
        pango_ctx: &pango::Context,
        theme: &Theme,
        cluster: &LayoutCluster,
    ) {
        let (l, t, _r, _b) = cluster_rect(cluster);
        let rect = graphene::Rect::new(l as f32, t as f32, cluster.width as f32, cluster.height as f32);
        fill_rect(snapshot, &rect, &theme.cluster_fill);
        stroke_rect(snapshot, &rect, &theme.cluster_stroke, 1.0, None);

        if !cluster.title.trim().is_empty() {
            draw_text(
                snapshot,
                pango_ctx,
                &cluster.title,
                cluster.title_label.x,
                cluster.title_label.y,
                &theme.text,
                self.text_style.font_family.as_deref(),
                self.text_style.font_size,
                true,
                HAlign::Center,
            );
        }
    }
}
