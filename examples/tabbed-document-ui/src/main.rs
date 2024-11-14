use std::path;
use std::path::PathBuf;
use slotmap::SlotMap;
use thiserror::Error;
use cushy::figures::units::{Lp, Px};
use cushy::App;
use cushy::value::{Dynamic};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use cushy::window::{PendingWindow};
use cushy::Open;
use cushy::styles::components::IntrinsicPadding;
use cushy::styles::Dimension;
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::app_tabs::new::NewTab;
use crate::app_tabs::TabKind;
use crate::config::Config;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::image::ImageDocument;
use crate::documents::text::TextDocument;
use crate::widgets::tab_bar::{TabBar, TabKey};

mod config;
mod widgets;
mod global_context;
mod context;
mod app_tabs;
mod documents;

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind>>,
    config: Dynamic<Config>,
    context: Dynamic<Context>,

    documents: Dynamic<SlotMap<DocumentKey, DocumentKind>>,
}

#[cushy::main]
fn main(app: &mut App) -> cushy::Result {

    let pending = PendingWindow::default();
    let window = pending.handle();

    let config = Dynamic::new(config::load());
    let documents = Dynamic::new(SlotMap::default());
    let tab_bar = Dynamic::new(TabBar::new());

    let mut context = Context::default();
    context.provide(config.clone());
    context.provide(documents.clone());
    context.provide(tab_bar.clone());
    context.provide(window);

    let context = Dynamic::new(context);

    let mut app_state = AppState {
        tab_bar: tab_bar.clone(),
        context: context.clone(),
        config,
        documents
    };

    let toolbar = make_toolbar(&mut app_state);

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(),
    ];

    let ui = pending.with_root(
        ui_elements
            .into_rows()
            .width(Px::new(800)..)
            .height(Px::new(600)..)
            .fit_vertically()
            .fit_horizontally()
            .make_widget()
    )
        .on_close({
            let config = app_state.config.clone();
            move ||{
                let config = config.lock();
                println!("Saving config");
                config::save(&*config);
            }
        })
        .titled("Tabbed document UI");



    if app_state.config.lock().show_home_on_startup {
        add_home_tab(&context, &app_state.tab_bar);
    }

    for path in app_state.config.lock().open_document_paths.clone() {
        open_document(&context, &app_state.documents, &app_state.tab_bar, path).ok();
    }


    ui.open(app)?;

    // FIXME control never returns here (at least on windows)

    Ok(())
}

#[derive(Error, Debug)]
enum OpenDocumentError {
    #[error("Unsupported file type. extension: {extension}")]
    UnsupportedFileExtension{extension: String},
    #[error("IO error, cause: {cause}")]
    IoError{cause: std::io::Error},
}

const SUPPORTED_IMAGE_EXTENSIONS: [&'static str; 5] = ["bmp", "png", "jpg", "jpeg", "svg"];
const SUPPORTED_TEXT_EXTENSIONS: [&'static str; 1] = ["txt"];

fn open_document(
    context: &Dynamic<Context>,
    documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>,
    tab_bar: &Dynamic<TabBar<TabKind>>,
    path: PathBuf
) -> Result<(), OpenDocumentError> {
    println!("open_document. path: {:?}", path);

    let path = path::absolute(path)
        .or_else(|cause| Err(OpenDocumentError::IoError { cause }))?;

    let extension = path.extension().unwrap().to_str().unwrap();

    let tab_key = if SUPPORTED_TEXT_EXTENSIONS.contains(&extension) {
        let document = DocumentKind::TextDocument(TextDocument::from_path(path));

        let tab_key = make_document_tab(context, documents, tab_bar, document);

        tab_key
    } else if SUPPORTED_IMAGE_EXTENSIONS.contains(&extension) {
        let document = DocumentKind::ImageDocument(ImageDocument::from_path(path));

        let tab_key = make_document_tab(context, documents, tab_bar, document);

        tab_key
    } else {
        return Err(OpenDocumentError::UnsupportedFileExtension { extension: extension.to_string() });
    };

    println!("added document tab with key. key: {:?}", tab_key);

    Ok(())
}

fn make_document_tab(context: &Dynamic<Context>, documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>, tab_bar: &Dynamic<TabBar<TabKind>>, document: DocumentKind) -> TabKey {
    let document_key = documents.lock().insert(document);

    let document_tab = DocumentTab::new(document_key);

    let mut tab_bar_guard = tab_bar.lock();
    let tab_key = tab_bar_guard.add_tab(context, TabKind::Document(document_tab));
    tab_key
}

fn make_toolbar(app_state: &mut AppState) -> Stack {
    let button_padding = Dimension::Lp(Lp::points(4));

    let home_button = "Home"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let context = app_state.context.clone();
            move |_|{
                println!("home clicked");

                add_home_tab(&context, &tab_bar);
            }
        })
        .with(&IntrinsicPadding, button_padding);

    let new_button = "New"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let context = app_state.context.clone();
            move |_|{
                println!("New clicked");

                add_new_tab(&context, &tab_bar)
            }
        })
        .with(&IntrinsicPadding, button_padding);

    let open_button = "Open"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let documents = app_state.documents.clone();
            let context = app_state.context.clone();
            move |_|{
                println!("open clicked");

                let path = PathBuf::from("examples/tabbed-document-ui/assets/text_file_1.txt");

                open_document(&context, &documents, &tab_bar, path).ok();
            }
        })
        .with(&IntrinsicPadding, button_padding);


    let close_all_button = "Close all"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            move |_| {
                println!("close all clicked");

                tab_bar.lock().close_all();
            }
        })
        .with(&IntrinsicPadding, button_padding);


    let toolbar_widgets: [WidgetInstance; 5] = [
        home_button.make_widget(),
        new_button.make_widget(),
        open_button.make_widget(),
        close_all_button.make_widget(),
        Expand::empty().make_widget(),
    ];

    let toolbar = toolbar_widgets.into_columns();
    toolbar
}

fn add_new_tab(context: &Dynamic<Context>, tab_bar: &Dynamic<TabBar<TabKind>>) {
    let mut tab_bar_guard = tab_bar
        .lock();

    tab_bar_guard
        .add_tab(context, TabKind::New(NewTab::default()));
}

fn add_home_tab(context: &Dynamic<Context>, tab_bar: &Dynamic<TabBar<TabKind>>) {
    let mut tab_bar_guard = tab_bar
        .lock();

    let home_tab_result = tab_bar_guard.with_tabs(|mut iter|{
        iter.find_map(move |(_key, tab)|
            match tab {
                TabKind::Home(tab) => Some((_key, tab.clone())),
                _ => None
            }
        )
    });

    if let Some((key, _tab)) = home_tab_result {
        tab_bar_guard.activate(key);
    } else {
        tab_bar_guard
            .add_tab(context, TabKind::Home(HomeTab::default()));
    }
}
