use gettextrs::gettext;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gio, glib};
use tracing::{debug, info};

use crate::config::{app_id, authors, pkgdatadir, profile, resources_file, version};
use crate::window::PikyApplicationWindow;

mod imp {
    use super::*;
    use glib::WeakRef;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct PikyApplication {
        pub window: OnceCell<WeakRef<PikyApplicationWindow>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PikyApplication {
        const NAME: &'static str = "PikyApplication";
        type Type = super::PikyApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for PikyApplication {}

    impl ApplicationImpl for PikyApplication {
        fn activate(&self) {
            debug!("AdwApplication<PikyApplication>::activate");
            self.parent_activate();
            let app = self.obj();

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = PikyApplicationWindow::new(&app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            app.main_window().present();
        }

        fn startup(&self) {
            debug!("AdwApplication<PikyApplication>::startup");
            self.parent_startup();
            let app = self.obj();

            app.setup_gactions();
            app.setup_accels();
        }
    }

    impl AdwApplicationImpl for PikyApplication {}

    impl GtkApplicationImpl for PikyApplication {}
}

glib::wrapper! {
    pub struct PikyApplication(ObjectSubclass<imp::PikyApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl PikyApplication {
    fn main_window(&self) -> PikyApplicationWindow {
        self.imp().window.get().unwrap().upgrade().unwrap()
    }

    fn setup_gactions(&self) {
        // Quit
        let action_quit = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| {
                // This is needed to trigger the delete event and saving the window state
                app.main_window().close();
                app.quit();
            })
            .build();

        // About
        let action_about = gio::ActionEntry::builder("about")
            .activate(|app: &Self, _, _| {
                app.show_about_dialog();
            })
            .build();
        self.add_action_entries([action_quit, action_about]);
    }

    // Sets up keyboard shortcuts
    fn setup_accels(&self) {
        self.set_accels_for_action("app.quit", &["<Control>q"]);
    }

    fn show_about_dialog(&self) {
        let dialog = adw::AboutDialog::builder()
            .application_name(gettext("Piky"))
            .application_icon(app_id())
            .version(version())
            .license_type(gtk::License::Custom)
            .license("European Union Public Licence 1.2 (EUPL-1.2)")
            .website("https://codeberg.org/anrouxel/piky")
            .issue_url("https://codeberg.org/anrouxel/piky/issues")
            .translator_credits(gettext("translator-credits"))
            .developers(authors())
            .build();

        dialog.present(Some(&self.main_window()));
    }

    pub fn run(&self) -> glib::ExitCode {
        info!("AdwApplication<PikyApplication>::run");
        info!("Version: {} ({})", version(), profile());
        info!("Datadir: {}", pkgdatadir());

        ApplicationExtManual::run(self)
    }
}

impl Default for PikyApplication {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", app_id())
            .property("resource-base-path", resources_file())
            .build()
    }
}
