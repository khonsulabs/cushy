use std::path::PathBuf;
use slotmap::SlotMap;
use cushy::dialog::FilePicker;
use cushy::figures::units::Px;
use cushy::styles::components::IntrinsicPadding;
use cushy::value::{Destination, Dynamic, Source, Validations};
use cushy::widget::{MakeWidget, WidgetInstance};
use cushy::widgets::{Button, Grid, Input, Space};
use cushy::widgets::grid::{GridDimension, GridWidgets};
use cushy::widgets::label::Displayable;
use cushy::window::WindowHandle;
use crate::app_tabs::document::{DocumentTab, DocumentTabMessage};
use crate::app_tabs::{TabKind, TabKindMessage};
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::image::ImageDocument;
use crate::documents::text::TextDocument;
use crate::task::Task;
use crate::widgets::tab_bar::{Tab, TabBar, TabKey};

#[derive(Clone, PartialEq)]
pub enum NewTabMessage {
    None,
    OkClicked,
    OkClickedOnValidForm,
}

impl Default for NewTabMessage {
    fn default() -> Self {
        Self::None
    }
}

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
    message: Dynamic<NewTabMessage>,
    validations: Validations,
}

impl NewTab {
    pub fn new(message: Dynamic<NewTabMessage>) -> Self {
        Self {
            message,
            ..Self::default()
        }
    }
}

impl Tab<NewTabMessage> for NewTab {
    fn label(&self, _context: &Dynamic<Context>) -> String {
        "New".to_string()
    }

    fn make_content(&self, context: &Dynamic<Context>, _tab_key: TabKey) -> WidgetInstance {
        let validations = self.validations.clone();

        let window = context.lock().with_context::<WindowHandle, _, _>(|window_handle| {
            window_handle.clone()
        }).unwrap();


        let name_label = "Name".into_label()
            .align_left();
        let name_input = Input::new(self.name.clone())
            .placeholder("Name without extension")
            .validation(validations.validate(&self.name.clone(), |name: &String| {
                if name.is_empty() {
                    Err("Cannot be empty")
                } else {
                    Ok(())
                }
            }))
            .hint("* required");

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
            .placeholder("Choose a directory")
            .validation(validations.validate(&self.directory.clone(), |path| {
                if !(path.is_dir() && path.exists())  {
                    Err("Must be a valid directory")
                } else {
                    Ok(())
                }
            }))
            .hint("* required")
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
            .centered()
            .validation(validations.validate(&self.kind, |kind|{
                if kind.is_none() {
                    Err("Required")
                } else {
                    Ok(())
                }
            }));

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
                let message = self.message.clone();
                move |_event| message.set(NewTabMessage::OkClicked)
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

    fn update(&mut self, context: &Dynamic<Context>, tab_key: TabKey, message: NewTabMessage) -> Task<NewTabMessage> {

        let documents = context.lock().with_context::<Dynamic<SlotMap<DocumentKey, DocumentKind>>, _, _>(|documents| {
            documents.clone()
        }).unwrap();

        let tab_bar = context.lock().with_context::<Dynamic<TabBar<TabKind, TabKindMessage>>, _, _>(|tab_bar| {
            tab_bar.clone()
        }).unwrap();

        match message {
            NewTabMessage::None => Task::none(),
            NewTabMessage::OkClicked => {
                if self.validations.is_valid() {
                    Task::done(NewTabMessage::OkClickedOnValidForm)
                } else {
                    Task::none()
                }
            }
            NewTabMessage::OkClickedOnValidForm => {
                Task::future({
                    let documents = documents.clone();
                    let tab_bar = tab_bar.clone();
                    let context = context.clone();
                    let kind = self.kind.clone();
                    let name = self.name.clone();
                    let directory = self.directory.clone();

                    async move {
                        let kind = kind.get();
                        let mut name = name.get();
                        let mut path = directory.get();

                        println!("kind: {:?}, name: {:?}, path: {:?}", kind, name, path);

                        match kind.unwrap() {
                            KindChoice::Text => {
                                name.push_str(".txt");
                                path.push(&name);

                                let document = DocumentKind::TextDocument(TextDocument::new(path.clone()));

                                let document_key = documents.lock().insert(document);
                                let document_tab = DocumentTab::new(document_key);

                                tab_bar.lock().replace(tab_key, &context, TabKind::Document(document_tab));
                            }
                            KindChoice::Image => {
                                name.push_str(".png");
                                path.push(&name);

                                let document = DocumentKind::ImageDocument(ImageDocument::new(path.clone()));

                                let document_key = documents.lock().insert(document);
                                let document_tab = DocumentTab::new(document_key);

                                tab_bar.lock().replace(tab_key, &context, TabKind::Document(document_tab));
                            }
                        }

                        // FIXME this not correct now since the tab has been replaced with a different type of TabKind and will
                        //       result in a panic when the message is processed.
                        NewTabMessage::None
                        //       we cannot do this, due to the return type:
                        // DocumentTabMessage::None

                        // So it seems that all this code needs to be moved up a layer so that the 'new' tab knows
                        // nothing about 'documents' or the 'tab_bar'
                    }
                })


            }
        }
    }
}