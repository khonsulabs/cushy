use std::path::PathBuf;
use slotmap::SlotMap;
use cushy::dialog::FilePicker;
use cushy::figures::units::Px;
use cushy::styles::components::IntrinsicPadding;
use cushy::value::{Destination, Dynamic, Source};
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::{Button, Grid, Input, Space};
use cushy::widgets::grid::{GridDimension, GridWidgets};
use cushy::widgets::label::Displayable;
use cushy::window::WindowHandle;
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::TabKind;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::text::TextDocument;
use crate::widgets::tab_bar::{Tab, TabBar};

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum KindChoice {
    #[default]
    Text,
    Image,
}
#[derive(Clone, Default)]
pub struct NewTab {
    name: Dynamic<String>,
    directory: Dynamic<PathBuf>,
    kind: Dynamic<Option<KindChoice>>,
}

impl Tab for NewTab {
    fn label(&self, _context: &Dynamic<Context>) -> String {
        "New".to_string()
    }

    fn make_content(&self, context: &Dynamic<Context>) -> WidgetInstance {


        let documents = context.lock().with_context::<Dynamic<SlotMap<DocumentKey, DocumentKind>>, _, _>(|documents| {
            documents.clone()
        }).unwrap();

        let tab_bar = context.lock().with_context::<Dynamic<TabBar<TabKind>>, _, _>(|tab_bar| {
            tab_bar.clone()
        }).unwrap();

        let window = context.lock().with_context::<WindowHandle, _, _>(|window_handle| {
            window_handle.clone()
        }).unwrap();


        let name_label = "Name".into_label()
            .align_left();
        let name_input = Input::new(self.name.clone());

        let name_row = (name_label, name_input);

        // FIXME remove this workaround for lack of grid gutter support.
        let gutter_row_1 = (
            Space::clear().height(Px::new(5)),
            Space::clear().height(Px::new(5))
        );

        let directory_label = "Directory".into_label();
        let directory_input = Input::new(self.directory.clone().map_each(|path|{
            path.to_str().unwrap().to_string()
        }))
            .expand_horizontally();

        let directory_button = Button::new("...")
            .on_click({
                let directory = self.directory.clone();

                move |_event| {
                    println!("on_click");

                    FilePicker::new()
                        .with_title("Choose folder")
                        .pick_folder(&window,{
                            // NOTE: Nested callbacks require a second clone
                            let directory = directory.clone();

                            move |path|{
                                if let Some(path) = path {
                                    println!("path: {:?}", path);
                                    directory.set(path.clone());
                                }
                            }
                        });
                }
            });

        let directory_input_and_button = directory_input
            .and(directory_button)
            .into_columns();

        let directory_row = (directory_label, directory_input_and_button);

        // FIXME remove this workaround for lack of grid gutter support.
        let gutter_row_2 = (
            Space::clear().height(Px::new(5)),
            Space::clear().height(Px::new(5))
        );

        let type_label = "Type".into_label();
        let type_choice = self.kind
            .new_radio(Some(KindChoice::Text))
            .labelled_by("Text")
            .and(self.kind.new_radio(Some(KindChoice::Image)).labelled_by("Image"))
            .into_columns()
            .centered();

        let type_row = (type_label, type_choice);

        let grid_widgets = GridWidgets::from(name_row)
            .and(gutter_row_1)
            .and(directory_row)
            .and(gutter_row_2)
            .and(type_row);

        let grid = Grid::from_rows(grid_widgets)
            .dimensions([
                GridDimension::FitContent,
                GridDimension::Fractional { weight: 1 },
            ])
            // FIXME failing to set a gutter between the rows
            .with(&IntrinsicPadding, Px::new(5)); // no visible effect.

        let ok_button = "Ok".into_button()
            .on_click({

                let kind = self.kind.clone();
                let documents = documents.clone();
                let tab_bar = tab_bar.clone();
                let context = context.clone();

                move |_event|{
                    println!("kind: {:?}", kind);

                    let document = DocumentKind::TextDocument(TextDocument::from_path(PathBuf::from("examples/tabbed-document-ui/assets/text_file_1.txt")));

                    let document_key = documents.lock().insert(document);

                    let document_tab = DocumentTab::new(document_key);

                    let mut tab_bar_guard = tab_bar.lock();

                    let _tab_key = tab_bar_guard.add_tab(&context, TabKind::Document(document_tab));
                }
            });

        let form = grid
            .and(ok_button)
            .into_rows();

        Space::clear()
            .expand_weighted(1)
            .and(form
                .expand_horizontally()
                .expand_weighted(8)
            )
            .and(Space::clear()
                .expand_weighted(1)
            )
            .into_columns()

            .make_widget()
    }
}