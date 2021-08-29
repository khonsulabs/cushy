//! Browser implementation of the List widget.
//!
//! Despite this widget being inspired by the <ol> and <ul> tags, the browser
//! implements lists adornments by utilizing the padding area, but causes many
//! issues. By switching to a table, we can achieve much more flexible and
//! consistent layout options.

use gooey_browser::{
    utils::{create_element, window_element_by_widget_id, CssBlockBuilder, CssRules},
    WebSys, WebSysTransmogrifier,
};
use gooey_core::{
    styles::style_sheet::State, Frontend, Transmogrifier, TransmogrifierContext, WidgetRegistration,
};
use wasm_bindgen::JsCast;
use web_sys::{HtmlDivElement, HtmlElement, HtmlTableElement};

use super::ItemLabelIterator;
use crate::list::{List, ListAdornmentSpacing, ListCommand, ListTransmogrifier};

impl Transmogrifier<WebSys> for ListTransmogrifier {
    type State = Option<CssRules>;
    type Widget = List;

    fn receive_command(
        &self,
        command: ListCommand,
        context: &mut TransmogrifierContext<'_, Self, WebSys>,
    ) {
        match command {
            ListCommand::ChildRemoved(child) => {
                // We need to remove the <tr>, not just the child.
                if let Some(tr) = window_element_by_widget_id::<HtmlElement>(child.id)
                    .and_then(|child| child.parent_node())
                    .and_then(|td| td.parent_node())
                {
                    let td = tr.parent_node().unwrap();
                    td.remove_child(&td).unwrap();
                }
            }
            ListCommand::ChildAdded(_child) => {
                if let Some(_table) =
                    window_element_by_widget_id::<HtmlDivElement>(context.registration.id().id)
                {
                    todo!(
                        "need to insert the value at the correct location, and then update the \
                         remaining indicators"
                    )
                    // materialize_child(&child, context, &container);
                }
            }
        }
    }
}

impl WebSysTransmogrifier for ListTransmogrifier {
    fn transmogrify(
        &self,
        context: TransmogrifierContext<'_, Self, WebSys>,
    ) -> Option<web_sys::HtmlElement> {
        let container = create_element::<HtmlTableElement>("table");
        container.set_attribute("aria-role", "list").unwrap();
        let effective_style = context
            .frontend
            .gooey()
            .stylesheet()
            .effective_style_for::<Self::Widget>(context.style().clone(), &State::default());
        let spacing = effective_style.get_or_default::<ListAdornmentSpacing>();
        let css = self
            .initialize_widget_element(&container, &context)
            .unwrap_or_default()
            .and(
                &CssBlockBuilder::for_id(context.registration.id().id)
                    .and_additional_selector(" td")
                    .with_css_statement("vertical-align: baseline")
                    .to_string(),
            )
            .and(
                &CssBlockBuilder::for_id(context.registration.id().id)
                    .and_additional_selector(" td.label")
                    .with_css_statement("text-align: right")
                    .with_css_statement(&format!("padding-right: {:.03}pt", spacing.0.get()))
                    .to_string(),
            );
        *context.state = Some(css);

        let mut labels =
            ItemLabelIterator::new(&context.widget.kind, context.widget.children.len());
        for child in context.widget.children.clone() {
            materialize_child(
                labels.next().unwrap().as_deref(),
                &child,
                &context,
                &container,
            );
        }
        Some(container.unchecked_into())
    }
}

fn materialize_child(
    item_label: Option<&str>,
    layout_child: &WidgetRegistration,
    context: &TransmogrifierContext<'_, ListTransmogrifier, WebSys>,
    container: &HtmlElement,
) {
    context.frontend.with_transmogrifier(
        layout_child.id(),
        |child_transmogrifier, mut child_context| {
            if let Some(child) = child_transmogrifier.transmogrify(&mut child_context) {
                let tr = create_element::<HtmlElement>("tr");
                tr.set_attribute("aria-role", "listitem").unwrap();

                if let Some(item_label) = item_label {
                    let indicator = create_element::<HtmlElement>("td");
                    indicator
                        .set_attribute("aria-role", "presentation")
                        .unwrap();
                    indicator.class_list().add_1("label").unwrap();
                    indicator.set_inner_text(item_label);
                    tr.append_child(&indicator)
                        .expect("error appending indicator");
                }

                let child_cell = create_element::<HtmlElement>("td");
                child_cell
                    .set_attribute("aria-role", "presentation")
                    .unwrap();
                child_cell
                    .append_child(&child)
                    .expect("error appending child");
                container.append_child(&tr).expect("error appending tr");
                tr.append_child(&child_cell)
                    .expect("error appending content");
            }
        },
    );
}
