use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::{gio, glib};
use tracing::debug;

use crate::application::PikyApplication;
use crate::config::app_id;
use crate::diagram_view::log_flowchart_layout;

const DEMO_DIAGRAM: &str = r#"
flowchart TD
    A[Start] --> B{Is it working?}
    B -- Yes --> C[Great]
    B -- No --> D[Debug]
    D --> B
"#;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct PikyApplicationWindow {
        pub header_bar: OnceCell<adw::HeaderBar>,
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

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);

        log_flowchart_layout(DEMO_DIAGRAM);

        self.set_title(Some(&gettext("Piky")));
        self.set_content(Some(&toolbar_view));

        imp.header_bar
            .set(header_bar)
            .expect("header_bar already set");
        imp.settings
            .set(gio::Settings::new(app_id()))
            .expect("settings already set");
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
