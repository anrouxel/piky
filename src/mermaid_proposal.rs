use gtk::glib;
use gtk::subclass::prelude::*;
use merman_editor_core::CompletionItem;

mod imp {
    use super::*;
    use std::cell::OnceCell;

    #[derive(Debug, Default)]
    pub struct MermaidProposal {
        pub item: OnceCell<CompletionItem>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MermaidProposal {
        const NAME: &'static str = "PikyMermaidCompletionProposal";
        type Type = super::MermaidProposal;
        type Interfaces = (sourceview5::CompletionProposal,);
    }

    impl ObjectImpl for MermaidProposal {}

    impl sourceview5::subclass::prelude::CompletionProposalImpl for MermaidProposal {}
}

glib::wrapper! {
    pub struct MermaidProposal(ObjectSubclass<imp::MermaidProposal>)
        @implements sourceview5::CompletionProposal;
}

impl MermaidProposal {
    pub fn new(item: CompletionItem) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp()
            .item
            .set(item)
            .expect("MermaidProposal item already initialized");
        obj
    }

    pub fn item(&self) -> &CompletionItem {
        self.imp()
            .item
            .get()
            .expect("MermaidProposal item not initialized")
    }
}
