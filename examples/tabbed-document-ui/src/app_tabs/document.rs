use slotmap::SlotMap;
use cushy::value::Dynamic;
use cushy::widget::WidgetInstance;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};

#[derive(Clone, Copy)]
pub struct DocumentTab {
    document_key: DocumentKey
}

impl DocumentTab {

    pub fn new(document_key: DocumentKey) -> Self {
        Self {
            document_key
        }
    }

    pub fn create_label(&self) -> String {
        "Document".to_string()
    }

    pub fn create_content(&self, context: &mut Context) -> WidgetInstance {

        context.with_context::<Dynamic<SlotMap<DocumentKey, DocumentKind>>, _, _>(|documents| {
            let documents_guard = documents.lock();
            let document = documents_guard.get(self.document_key).unwrap();

            match document {
                DocumentKind::TextDocument(text_document) => text_document.create_content()
            }
        }).unwrap()
    }
}