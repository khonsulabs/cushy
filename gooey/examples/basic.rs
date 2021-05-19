use gooey::{
    core::{styles::Style, Frontend, Gooey},
    widgets::{button::Button, container::Container},
};

pub fn ui<F: Frontend>() -> Gooey<F> {
    Gooey::with(|storage| {
        Container::new(
            Button {
                label: String::from("Hello, World"),
                style: Style::default(),
            },
            storage,
        )
    })
}

fn main() {
    gooey::main(ui())
}
