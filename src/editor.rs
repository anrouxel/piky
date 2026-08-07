use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::glib;
use sourceview5::prelude::*;
use tracing::warn;

use crate::mermaid_completion::MermaidCompletionProvider;

const LANGUAGE_ID: &str = "mermaid";
const LANGUAGE_SPECS_RESOURCE: &str = "resource:///eu/anrouxel/piky/language-specs";

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct PikyEditor {
        pub view: OnceCell<sourceview5::View>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PikyEditor {
        const NAME: &'static str = "PikyEditor";
        type Type = super::PikyEditor;
        type ParentType = adw::Bin;
    }

    impl ObjectImpl for PikyEditor {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().setup_widgets();
        }
    }

    impl WidgetImpl for PikyEditor {}
    impl BinImpl for PikyEditor {}
}

glib::wrapper! {
    pub struct PikyEditor(ObjectSubclass<imp::PikyEditor>)
        @extends gtk::Widget, adw::Bin,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl PikyEditor {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn buffer(&self) -> sourceview5::Buffer {
        self.view()
            .buffer()
            .downcast::<sourceview5::Buffer>()
            .expect("PikyEditor view buffer is a sourceview5::Buffer")
    }

    pub fn text(&self) -> glib::GString {
        let buffer = self.buffer();
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), true)
    }

    fn view(&self) -> &sourceview5::View {
        self.imp().view.get().expect("PikyEditor view initialized")
    }

    fn setup_widgets(&self) {
        let language_manager = sourceview5::LanguageManager::default();
        language_manager.prepend_search_path(LANGUAGE_SPECS_RESOURCE);
        let language = language_manager.language(LANGUAGE_ID);
        if language.is_none() {
            warn!("Mermaid language spec \"{LANGUAGE_ID}\" not found in {LANGUAGE_SPECS_RESOURCE}");
        }

        let buffer = sourceview5::Buffer::new(None);
        buffer.set_language(language.as_ref());
        buffer.set_highlight_syntax(true);
        if let Some(scheme) = self.preferred_style_scheme() {
            buffer.set_style_scheme(Some(&scheme));
        }

        let view = sourceview5::View::with_buffer(&buffer);
        view.set_monospace(true);
        view.set_show_line_numbers(true);
        view.set_highlight_current_line(true);
        view.set_auto_indent(true);
        view.set_tab_width(2);
        view.set_insert_spaces_instead_of_tabs(true);
        view.set_wrap_mode(gtk::WrapMode::WordChar);

        view.completion()
            .add_provider(&MermaidCompletionProvider::new());

        let scrolled_window = gtk::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .child(&view)
            .build();

        self.set_child(Some(&scrolled_window));

        self.imp()
            .view
            .set(view)
            .expect("PikyEditor view already initialized");
    }

    fn preferred_style_scheme(&self) -> Option<sourceview5::StyleScheme> {
        let scheme_manager = sourceview5::StyleSchemeManager::default();
        let id = if adw::StyleManager::default().is_dark() {
            "Adwaita-dark"
        } else {
            "Adwaita"
        };
        scheme_manager.scheme(id)
    }
}

impl Default for PikyEditor {
    fn default() -> Self {
        Self::new()
    }
}
