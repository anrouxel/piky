use gtk::TextIter;
use gtk::{gio, glib};
use merman_editor_core::{
    CompletionInsertTextFormat, DocumentKind, DocumentWorkspace, Position, completion_for_snapshot,
};
use sourceview5::prelude::*;

use crate::mermaid_proposal::MermaidProposal;

const DOCUMENT_URI: &str = "file:///piky-buffer.mmd";

mod imp {
    use super::*;
    use glib::subclass::prelude::*;
    use std::cell::{Cell, RefCell};

    #[derive(Debug)]
    pub struct MermaidCompletionProvider {
        pub workspace: RefCell<DocumentWorkspace>,
        pub version: Cell<i32>,
    }

    impl Default for MermaidCompletionProvider {
        fn default() -> Self {
            Self {
                workspace: RefCell::new(DocumentWorkspace::new()),
                version: Cell::new(0),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MermaidCompletionProvider {
        const NAME: &'static str = "PikyMermaidCompletionProvider";
        type Type = super::MermaidCompletionProvider;
        type Interfaces = (sourceview5::CompletionProvider,);
    }

    impl ObjectImpl for MermaidCompletionProvider {}

    impl sourceview5::subclass::prelude::CompletionProviderImpl for MermaidCompletionProvider {
        fn title(&self) -> Option<glib::GString> {
            Some("Mermaid".into())
        }

        fn priority(&self, _context: &sourceview5::CompletionContext) -> i32 {
            0
        }

        fn is_trigger(&self, _iter: &gtk::TextIter, c: char) -> bool {
            c.is_alphanumeric() || matches!(c, '-' | '>' | '<' | ':' | '(' | ')' | '.' | '_' | '%')
        }

        fn populate(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> Result<gio::ListModel, glib::Error> {
            let store = gio::ListStore::new::<MermaidProposal>();

            let (Some(buffer), Some((_, cursor_iter))) = (context.buffer(), context.bounds())
            else {
                return Ok(store.upcast());
            };

            let text = buffer
                .text(&buffer.start_iter(), &buffer.end_iter(), true)
                .to_string();
            let position = position_for_iter(&buffer, &cursor_iter);

            let version = self.version.get() + 1;
            self.version.set(version);

            let snapshot = self.workspace.borrow_mut().upsert(
                DOCUMENT_URI,
                version,
                text,
                DocumentKind::Diagram,
            );

            for item in completion_for_snapshot(&snapshot, position).items {
                store.append(&MermaidProposal::new(item));
            }

            Ok(store.upcast())
        }

        fn display(
            &self,
            _context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            let Some(proposal) = proposal.downcast_ref::<MermaidProposal>() else {
                return;
            };
            let item = proposal.item();

            match cell.column() {
                sourceview5::CompletionColumn::TypedText => {
                    cell.set_text(Some(item.label.as_str()))
                }
                sourceview5::CompletionColumn::Details | sourceview5::CompletionColumn::Comment => {
                    cell.set_text(item.detail.as_deref())
                }
                _ => {}
            }
        }

        fn activate(
            &self,
            context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
        ) {
            let Some(proposal) = proposal.downcast_ref::<MermaidProposal>() else {
                return;
            };
            let Some(buffer) = context.buffer() else {
                return;
            };
            let item = proposal.item();

            buffer.begin_user_action();

            if let Some(text_edit) = &item.text_edit {
                let mut start = iter_for_position(&buffer, text_edit.range.start);
                let mut end = iter_for_position(&buffer, text_edit.range.end);
                let text = resolved_insert_text(&text_edit.new_text, item.insert_text_format);
                buffer.delete(&mut start, &mut end);
                buffer.insert(&mut start, &text);
                buffer.place_cursor(&start);
            } else if let (Some(insert_text), Some((_, mut cursor_iter))) =
                (&item.insert_text, context.bounds())
            {
                let text = resolved_insert_text(insert_text, item.insert_text_format);
                buffer.insert(&mut cursor_iter, &text);
            }

            buffer.end_user_action();
        }
    }
}

glib::wrapper! {
    pub struct MermaidCompletionProvider(ObjectSubclass<imp::MermaidCompletionProvider>)
        @implements sourceview5::CompletionProvider;
}

impl MermaidCompletionProvider {
    pub fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for MermaidCompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts a text-buffer position into merman's UTF-16-based `Position`,
/// matching the LSP convention `completion_for_snapshot` expects.
fn position_for_iter(buffer: &sourceview5::Buffer, iter: &TextIter) -> Position {
    let line = iter.line();
    let Some(line_start) = buffer.iter_at_line(line) else {
        return Position::new(line.max(0) as usize, 0);
    };
    let prefix = buffer.text(&line_start, iter, true);
    Position::new(line.max(0) as usize, prefix.encode_utf16().count())
}

/// Converts merman's UTF-16-based `Position` back into a text-buffer iterator.
fn iter_for_position(buffer: &sourceview5::Buffer, position: Position) -> TextIter {
    let line = position.line as i32;
    let Some(line_start) = buffer.iter_at_line(line) else {
        return buffer.end_iter();
    };
    let mut line_end = line_start;
    line_end.forward_to_line_end();
    let line_text = buffer.text(&line_start, &line_end, true);

    let mut utf16_seen = 0usize;
    let mut char_offset = 0i32;
    for ch in line_text.chars() {
        if utf16_seen >= position.character {
            break;
        }
        utf16_seen += ch.len_utf16();
        char_offset += 1;
    }

    buffer
        .iter_at_line_offset(line, char_offset)
        .unwrap_or(line_end)
}

/// Strips LSP-style snippet placeholders (`${1:name}`, `$1`, `$0`) down to
/// plain text, since proposals are inserted without tab-stop support.
fn resolved_insert_text(text: &str, format: CompletionInsertTextFormat) -> String {
    if format == CompletionInsertTextFormat::PlainText {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '$' {
            result.push(ch);
            continue;
        }

        match chars.peek() {
            Some('{') => {
                chars.next();
                let mut placeholder = String::new();
                for inner in chars.by_ref() {
                    if inner == '}' {
                        break;
                    }
                    placeholder.push(inner);
                }
                if let Some(colon) = placeholder.find(':') {
                    result.push_str(&placeholder[colon + 1..]);
                }
            }
            Some(c) if c.is_ascii_digit() => {
                while matches!(chars.peek(), Some(c) if c.is_ascii_digit()) {
                    chars.next();
                }
            }
            _ => result.push('$'),
        }
    }

    result
}
