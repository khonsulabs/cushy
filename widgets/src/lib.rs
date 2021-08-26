//! Built-in widgets for user interfaces.

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    // TODO missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::if_not_else,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::missing_errors_doc, // TODO clippy::missing_errors_doc
    clippy::missing_panics_doc, // TODO clippy::missing_panics_doc
)]
#![cfg_attr(doc, warn(rustdoc::all))]

pub mod button;
pub mod checkbox;
pub mod component;
pub mod container;
pub mod form;
pub mod input;
pub mod label;
pub mod layout;
pub mod list;
pub mod navigator;

#[cfg(feature = "frontend-rasterizer")]
pub mod rasterized {
    use gooey_core::Transmogrifiers;
    use gooey_rasterizer::{make_rasterized, Rasterizer, Renderer};

    use crate::{
        button::ButtonTransmogrifier, checkbox::CheckboxTransmogrifier,
        container::ContainerTransmogrifier, input::InputTransmogrifier, label::LabelTransmogrifier,
        layout::LayoutTransmogrifier, list::ListTransmogrifier,
    };

    pub fn register_transmogrifiers<R: Renderer>(
        transmogrifiers: &mut Transmogrifiers<Rasterizer<R>>,
    ) {
        drop(transmogrifiers.register_transmogrifier(ButtonTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(ContainerTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(LabelTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(LayoutTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(CheckboxTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(InputTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(ListTransmogrifier));
    }

    #[must_use]
    pub fn default_transmogrifiers<R: Renderer>() -> Transmogrifiers<Rasterizer<R>> {
        let mut transmogrifiers = Transmogrifiers::default();
        register_transmogrifiers(&mut transmogrifiers);
        transmogrifiers
    }

    make_rasterized!(ButtonTransmogrifier);
    make_rasterized!(ContainerTransmogrifier);
    make_rasterized!(LabelTransmogrifier);
    make_rasterized!(LayoutTransmogrifier);
    make_rasterized!(CheckboxTransmogrifier);
    make_rasterized!(InputTransmogrifier);
    make_rasterized!(ListTransmogrifier);
}

#[cfg(feature = "frontend-browser")]
pub mod browser {
    use gooey_browser::{make_browser, WebSys};
    use gooey_core::Transmogrifiers;

    use crate::{
        button::ButtonTransmogrifier, checkbox::CheckboxTransmogrifier,
        container::ContainerTransmogrifier, input::InputTransmogrifier, label::LabelTransmogrifier,
        layout::LayoutTransmogrifier, list::ListTransmogrifier,
    };

    pub fn register_transmogrifiers(transmogrifiers: &mut Transmogrifiers<WebSys>) {
        drop(transmogrifiers.register_transmogrifier(ButtonTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(ContainerTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(LabelTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(LayoutTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(CheckboxTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(InputTransmogrifier));
        drop(transmogrifiers.register_transmogrifier(ListTransmogrifier));
    }

    #[must_use]
    pub fn default_transmogrifiers() -> Transmogrifiers<WebSys> {
        let mut transmogrifiers = Transmogrifiers::default();
        register_transmogrifiers(&mut transmogrifiers);
        transmogrifiers
    }

    make_browser!(ButtonTransmogrifier);
    make_browser!(ContainerTransmogrifier);
    make_browser!(LabelTransmogrifier);
    make_browser!(LayoutTransmogrifier);
    make_browser!(CheckboxTransmogrifier);
    make_browser!(InputTransmogrifier);
    make_browser!(ListTransmogrifier);
}
