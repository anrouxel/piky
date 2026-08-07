mod application;
mod config;
mod diagram_view;
mod i18n;
mod window;

use gtk::{gio, glib};

use crate::application::PikyApplication;

fn main() -> glib::ExitCode {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    i18n::init();

    let res = gio::Resource::load(config::resources_file()).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = PikyApplication::default();
    app.run()
}
