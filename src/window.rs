use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use tracing::{debug, warn};

use crate::application::PikyApplication;
use crate::canvas::Canvas;
use crate::config::app_id;

/// Shown until an "Open" action / recent-file support exists.
const DEMO_SOURCE: &str = r#"classDiagram
    namespace Shapes {
        class Shape {
            <<interface>>
            +draw() void
        }
        class Circle {
            -float radius
            +draw() void
        }
        class Square {
            -float side
            +draw() void
        }
    }
    class Renderer {
        -List~Shape~ shapes
        +render() void
    }
    Renderer o-- Shape : uses
    Circle ..|> Shape : implements
    Square ..|> Shape : implements
    Renderer --> Circle : depends on
    note for Renderer "Entry point of the app"
"#;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct PikyApplicationWindow {
        pub header_bar: OnceCell<adw::HeaderBar>,
        pub toast_overlay: OnceCell<adw::ToastOverlay>,
        pub canvas: OnceCell<Canvas>,
        pub settings: OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PikyApplicationWindow {
        const NAME: &'static str = "PikyApplicationWindow";
        type Type = super::PikyApplicationWindow;
        type ParentType = adw::ApplicationWindow;
    }

    impl ObjectImpl for PikyApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            obj.setup_widgets();
            obj.load_window_size();
        }
    }

    impl WidgetImpl for PikyApplicationWindow {}

    impl WindowImpl for PikyApplicationWindow {
        fn close_request(&self) -> glib::Propagation {
            if let Err(err) = self.obj().save_window_size() {
                tracing::warn!("Failed to save window state, {}", &err);
            }
            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for PikyApplicationWindow {}
    impl AdwApplicationWindowImpl for PikyApplicationWindow {}
}

glib::wrapper! {
    pub struct PikyApplicationWindow(ObjectSubclass<imp::PikyApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup,
                    gtk::Root, gtk::Native, gtk::ShortcutManager,
                    gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PikyApplicationWindow {
    pub fn new(app: &PikyApplication) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn setup_widgets(&self) {
        let imp = self.imp();

        let header_bar = adw::HeaderBar::new();

        let open_button = gtk::Button::from_icon_name("document-open-symbolic");
        open_button.set_tooltip_text(Some(&gettext("Open Diagram")));
        open_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| window.open_file_dialog()
        ));
        header_bar.pack_start(&open_button);

        let zoom_fit_button = gtk::Button::from_icon_name("zoom-fit-best-symbolic");
        zoom_fit_button.set_tooltip_text(Some(&gettext("Zoom to Fit")));
        zoom_fit_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                if let Some(canvas) = window.imp().canvas.get() {
                    canvas.zoom_to_fit();
                }
            }
        ));
        header_bar.pack_start(&zoom_fit_button);

        let canvas = Canvas::default();
        if let Err(err) = canvas.set_source(DEMO_SOURCE) {
            warn!("Failed to load demo diagram: {err}");
        }

        let toast_overlay = adw::ToastOverlay::new();
        toast_overlay.set_child(Some(&canvas));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&toast_overlay));

        self.set_title(Some(&gettext("Piky")));
        self.set_content(Some(&toolbar_view));

        imp.header_bar
            .set(header_bar)
            .expect("header_bar already set");
        imp.toast_overlay
            .set(toast_overlay)
            .expect("toast_overlay already set");
        imp.canvas.set(canvas).expect("canvas already set");
        imp.settings
            .set(gio::Settings::new(app_id()))
            .expect("settings already set");

        // The canvas has no real allocation yet during the first layout pass,
        // so `zoom_to_fit` (called eagerly above) can't frame the diagram
        // correctly. Redo it once the window has been laid out.
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                if let Some(canvas) = window.imp().canvas.get() {
                    canvas.zoom_to_fit();
                }
            }
        ));
    }

    fn open_file_dialog(&self) {
        let dialog = gtk::FileDialog::builder()
            .title(gettext("Open Mermaid Diagram"))
            .build();

        dialog.open(
            Some(self),
            gio::Cancellable::NONE,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |result| {
                    let Ok(file) = result else {
                        // User cancelled the dialog; nothing to report.
                        return;
                    };
                    match file.load_contents(gio::Cancellable::NONE) {
                        Ok((bytes, _)) => {
                            let text = String::from_utf8_lossy(&bytes).into_owned();
                            window.load_source(&text);
                        }
                        Err(err) => window.show_error(&err.to_string()),
                    }
                }
            ),
        );
    }

    fn load_source(&self, source: &str) {
        let Some(canvas) = self.imp().canvas.get() else {
            return;
        };
        if let Err(err) = canvas.set_source(source) {
            self.show_error(&err.to_string());
        }
    }

    fn show_error(&self, message: &str) {
        warn!("{message}");
        if let Some(overlay) = self.imp().toast_overlay.get() {
            overlay.add_toast(adw::Toast::new(message));
        }
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = self.imp().settings.get().unwrap();
        let (width, height) = self.default_size();

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;
        settings.set_boolean("is-maximized", self.is_maximized())?;

        debug!(
            "Saving window size: {}x{}, maximized: {}",
            width,
            height,
            self.is_maximized()
        );

        Ok(())
    }

    fn load_window_size(&self) {
        let settings = self.imp().settings.get().unwrap();
        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        debug!(
            "Loading window size: {}x{}, maximized: {}",
            width, height, is_maximized
        );

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }
}
