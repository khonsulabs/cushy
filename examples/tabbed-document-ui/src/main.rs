use std::path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use slotmap::SlotMap;
use thiserror::Error;
use cushy::figures::units::Px;
use cushy::Run;
use cushy::value::{Dynamic};
use cushy::widget::{IntoWidgetList, MakeWidget, WidgetInstance};
use cushy::widgets::{Expand, Stack};
use crate::app_tabs::document::DocumentTab;
use crate::app_tabs::home::HomeTab;
use crate::app_tabs::TabKind;
use crate::config::Config;
use crate::context::Context;
use crate::documents::{DocumentKey, DocumentKind};
use crate::documents::text::TextDocument;
use crate::widgets::tab_bar::TabBar;

mod config;
mod widgets;
mod global_context;
mod context;
mod app_tabs;
mod documents;

struct AppState {
    tab_bar: Dynamic<TabBar<TabKind>>,
    config: Dynamic<Config>,
    context: Arc<Mutex<Context>>,

    documents: Dynamic<SlotMap<DocumentKey, DocumentKind>>,
}

fn main() -> cushy::Result {

    let config = Dynamic::new(config::load());
    let documents = Dynamic::new(SlotMap::default());

    let mut context = Context::default();
    context.provide(config.clone());
    context.provide(documents.clone());

    let tab_bar = Dynamic::new(make_tab_bar());

    let mut app_state = AppState {
        tab_bar: tab_bar.clone(),
        context: Arc::new(Mutex::new(context)),
        config,
        documents
    };

    let toolbar = make_toolbar(&mut app_state);

    let ui_elements = [
        toolbar.make_widget(),
        app_state.tab_bar.lock().make_widget(&mut app_state.context),
    ];

    let ui = ui_elements
        .into_rows()
        .width(Px::new(800)..)
        .height(Px::new(600)..)
        .fit_vertically()
        .fit_horizontally()
        .into_window()
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
        add_home_tab(&app_state.tab_bar);
    }

    for path in app_state.config.lock().open_document_paths.clone() {
        open_document(&app_state.documents, &app_state.tab_bar, path).ok();
    }

    let cushy_result = ui.run();

    // FIXME control never returns here (at least on windows)

    cushy_result
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
    documents: &Dynamic<SlotMap<DocumentKey, DocumentKind>>,
    tab_bar: &Dynamic<TabBar<TabKind>>,
    path: PathBuf
) -> Result<(), OpenDocumentError> {
    println!("open_document. path: {:?}", path);

    let path = path::absolute(path)
        .or_else(|cause| Err(OpenDocumentError::IoError { cause }))?;

    let extension = path.extension().unwrap().to_str().unwrap();

    if SUPPORTED_TEXT_EXTENSIONS.contains(&extension) {
        let document = DocumentKind::TextDocument(TextDocument::from_path(path));
        let document_key = documents.lock().insert(document);

        let document_tab = DocumentTab::new(document_key);

        let mut tab_bar_guard = tab_bar.lock();
        let tab_key = tab_bar_guard.add_tab(TabKind::Document(document_tab));
        println!("added document tab with key. key: {:?}", tab_key);
    } else if SUPPORTED_IMAGE_EXTENSIONS.contains(&extension) {
        // TODO support images
        return Err(OpenDocumentError::UnsupportedFileExtension { extension: extension.to_string() });
    } else {
        return Err(OpenDocumentError::UnsupportedFileExtension { extension: extension.to_string() });
    }


    Ok(())
}

fn make_tab_bar() -> TabBar<TabKind> {
    TabBar::new()
}

fn make_toolbar(app_state: &mut AppState) -> Stack {
    let home_button = "Home"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            move |_|{
                println!("home clicked");

                add_home_tab(&tab_bar);
            }
        });

    let new_button = "New"
        .into_button()
        .on_click({
            let _tab_bar = app_state.tab_bar.clone();
            move |_|{
                println!("New clicked");
            }
        });

    let open_button = "Open"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            let documents = app_state.documents.clone();
            move |_|{
                println!("open clicked");

                let path = PathBuf::from("examples/tabbed-document-ui/assets/text_file_1.txt");

                open_document(&documents, &tab_bar, path).ok();
            }
        });


    let close_all_button = "Close all"
        .into_button()
        .on_click({
            let tab_bar = app_state.tab_bar.clone();
            move |_| {
                println!("close all clicked");

                tab_bar.lock().close_all();
            }
        });


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

fn add_home_tab(tab_bar: &Dynamic<TabBar<TabKind>>) {
    let mut tab_bar_guard = tab_bar
        .lock();

    let home_tab_result = tab_bar_guard.with_tabs(|mut iter|{
        iter.find_map(move |(_key, (tab, _state))|
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
            .add_tab(TabKind::Home(HomeTab::default()));
    }
}
