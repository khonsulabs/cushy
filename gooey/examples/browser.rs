use gooey::frontends::browser::WebSys;

mod shared;

fn main() {
    WebSys::new(shared::ui()).install_in_id("gooey")
}
