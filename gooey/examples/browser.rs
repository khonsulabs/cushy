use gooey::{core::Gooey, frontends::browser::WebSys, widgets::button::Button};

fn main() {
    WebSys::new(Gooey::new(Button {
        label: String::from("Hello"),
        disabled: false,
    }))
    .install_in_id("gooey")
}
