use gooey::{
    core::styles::Style,
    widgets::{button::Button, container::Container},
};

fn main() {
    gooey::main(|storage| {
        Container::new(
            Button {
                label: String::from("Hello, World"),
                style: Style::default(),
            },
            storage,
        )
    })
}
