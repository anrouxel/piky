//! GTK4/GSK canvas widget: draws a [`crate::diagram::Diagram`], and supports
//! panning (drag), zooming (scroll wheel + pinch), and click-to-log picking.

use gtk::glib;
use gtk::graphene;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use tracing::info;

use crate::diagram::{Diagram, DiagramError, Theme};

const MIN_SCALE: f64 = 0.05;
const MAX_SCALE: f64 = 12.0;
const ZOOM_STEP: f64 = 1.1;
const CLICK_DRAG_TOLERANCE_PX: f64 = 4.0;
const EDGE_HIT_TOLERANCE_PX: f64 = 6.0;
const FIT_MARGIN_PX: f64 = 32.0;

mod imp {
    use super::*;
    use std::cell::{Cell, RefCell};

    pub struct Canvas {
        pub diagram: RefCell<Option<Diagram>>,
        pub theme: RefCell<Theme>,
        pub scale: Cell<f64>,
        pub translate_x: Cell<f64>,
        pub translate_y: Cell<f64>,
        pub drag_start_translate: Cell<(f64, f64)>,
        pub drag_max_offset: Cell<f64>,
        pub pointer: Cell<(f64, f64)>,
        pub zoom_start_scale: Cell<f64>,
    }

    impl Default for Canvas {
        fn default() -> Self {
            Self {
                diagram: RefCell::new(None),
                theme: RefCell::new(Theme::default()),
                scale: Cell::new(1.0),
                translate_x: Cell::new(0.0),
                translate_y: Cell::new(0.0),
                drag_start_translate: Cell::new((0.0, 0.0)),
                drag_max_offset: Cell::new(0.0),
                pointer: Cell::new((0.0, 0.0)),
                zoom_start_scale: Cell::new(1.0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Canvas {
        const NAME: &'static str = "PikyCanvas";
        type Type = super::Canvas;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Canvas {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.set_hexpand(true);
            obj.set_vexpand(true);
            obj.set_can_focus(true);
            obj.setup_controllers();
        }
    }

    impl WidgetImpl for Canvas {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let width = obj.width() as f64;
            let height = obj.height() as f64;
            if width <= 0.0 || height <= 0.0 {
                return;
            }

            let theme = self.theme.borrow();
            snapshot.append_color(
                &theme.background,
                &graphene::Rect::new(0.0, 0.0, width as f32, height as f32),
            );

            let diagram_ref = self.diagram.borrow();
            let Some(diagram) = diagram_ref.as_ref() else {
                return;
            };

            let scale = self.scale.get();
            let (tx, ty) = (self.translate_x.get(), self.translate_y.get());

            snapshot.save();
            snapshot.translate(&graphene::Point::new(tx as f32, ty as f32));
            snapshot.scale(scale as f32, scale as f32);

            let pango_ctx = obj.pango_context();
            diagram.draw(snapshot, &pango_ctx, &theme);

            snapshot.restore();
        }
    }
}

glib::wrapper! {
    pub struct Canvas(ObjectSubclass<imp::Canvas>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Canvas {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Canvas {
    pub fn set_source(&self, source: &str) -> Result<(), DiagramError> {
        let diagram = Diagram::parse(source)?;
        *self.imp().diagram.borrow_mut() = Some(diagram);
        self.zoom_to_fit();
        self.queue_draw();
        Ok(())
    }

    /// Recomputes pan/zoom so the whole diagram is visible, based on the
    /// widget's current allocation. Best called once the widget has been
    /// allocated a real size (e.g. via `glib::idle_add_local_once` after the
    /// window is presented).
    pub fn zoom_to_fit(&self) {
        let imp = self.imp();
        let diagram_ref = imp.diagram.borrow();
        let Some(diagram) = diagram_ref.as_ref() else {
            return;
        };
        let (min_x, min_y, max_x, max_y) = diagram.bounds();
        let content_w = (max_x - min_x).max(1.0);
        let content_h = (max_y - min_y).max(1.0);

        let width = self.width() as f64;
        let height = self.height() as f64;
        let (avail_w, avail_h) = if width > 1.0 && height > 1.0 {
            (width, height)
        } else {
            (800.0, 600.0)
        };

        let scale = ((avail_w - FIT_MARGIN_PX * 2.0) / content_w)
            .min((avail_h - FIT_MARGIN_PX * 2.0) / content_h)
            .clamp(MIN_SCALE, MAX_SCALE);

        let tx = avail_w / 2.0 - (min_x + content_w / 2.0) * scale;
        let ty = avail_h / 2.0 - (min_y + content_h / 2.0) * scale;

        imp.scale.set(scale);
        imp.translate_x.set(tx);
        imp.translate_y.set(ty);
        self.queue_draw();
    }

    fn screen_to_diagram(&self, sx: f64, sy: f64) -> (f64, f64) {
        let imp = self.imp();
        let scale = imp.scale.get();
        ((sx - imp.translate_x.get()) / scale, (sy - imp.translate_y.get()) / scale)
    }

    fn zoom_at(&self, factor: f64, sx: f64, sy: f64) {
        let imp = self.imp();
        let old_scale = imp.scale.get();
        let new_scale = (old_scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let actual = new_scale / old_scale;

        let tx = imp.translate_x.get();
        let ty = imp.translate_y.get();
        imp.translate_x.set(sx - (sx - tx) * actual);
        imp.translate_y.set(sy - (sy - ty) * actual);
        imp.scale.set(new_scale);
        self.queue_draw();
    }

    fn handle_click(&self, sx: f64, sy: f64) {
        let imp = self.imp();
        let (dx, dy) = self.screen_to_diagram(sx, sy);
        let tolerance = EDGE_HIT_TOLERANCE_PX / imp.scale.get();

        let element = imp
            .diagram
            .borrow()
            .as_ref()
            .and_then(|diagram| diagram.hit_test(dx, dy, tolerance));

        match element {
            Some(element) => info!(?element, "diagram element clicked"),
            None => info!("click on empty canvas"),
        }
    }

    fn setup_controllers(&self) {
        let motion = gtk::EventControllerMotion::new();
        motion.connect_motion(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            move |_, x, y| {
                canvas.imp().pointer.set((x, y));
            }
        ));
        self.add_controller(motion);

        let drag = gtk::GestureDrag::new();
        drag.connect_drag_begin(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            move |_, _, _| {
                let imp = canvas.imp();
                imp.drag_start_translate
                    .set((imp.translate_x.get(), imp.translate_y.get()));
                imp.drag_max_offset.set(0.0);
            }
        ));
        drag.connect_drag_update(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            move |_, offset_x, offset_y| {
                let imp = canvas.imp();
                let (start_x, start_y) = imp.drag_start_translate.get();
                imp.translate_x.set(start_x + offset_x);
                imp.translate_y.set(start_y + offset_y);
                let magnitude = offset_x.hypot(offset_y);
                if magnitude > imp.drag_max_offset.get() {
                    imp.drag_max_offset.set(magnitude);
                }
                canvas.queue_draw();
            }
        ));
        drag.connect_drag_end(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            move |gesture, offset_x, offset_y| {
                let imp = canvas.imp();
                let magnitude = offset_x.hypot(offset_y).max(imp.drag_max_offset.get());
                if magnitude <= CLICK_DRAG_TOLERANCE_PX
                    && let Some((start_x, start_y)) = gesture.start_point() {
                        canvas.handle_click(start_x, start_y);
                    }
            }
        ));
        self.add_controller(drag);

        let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        scroll.connect_scroll(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, _dx, dy| {
                let (px, py) = canvas.imp().pointer.get();
                let factor = if dy < 0.0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
                canvas.zoom_at(factor, px, py);
                glib::Propagation::Stop
            }
        ));
        self.add_controller(scroll);

        let zoom = gtk::GestureZoom::new();
        zoom.connect_begin(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            move |_, _| {
                canvas.imp().zoom_start_scale.set(canvas.imp().scale.get());
            }
        ));
        zoom.connect_scale_changed(glib::clone!(
            #[weak(rename_to = canvas)]
            self,
            move |gesture, delta| {
                let imp = canvas.imp();
                let target = imp.zoom_start_scale.get() * delta;
                let factor = target / imp.scale.get();
                let (cx, cy) = gesture
                    .bounding_box_center()
                    .unwrap_or_else(|| imp.pointer.get());
                canvas.zoom_at(factor, cx, cy);
            }
        ));
        self.add_controller(zoom);
    }
}
