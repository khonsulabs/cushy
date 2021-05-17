use gooey::{
    core::{Frontend, Gooey},
    widgets::{button::Button, container::Container},
};

pub fn ui<F: Frontend>() -> Gooey<F> {
    Gooey::new(Container::new(Button {
        label: String::from("Hello"),
        disabled: false,
    }))
}
