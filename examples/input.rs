use gooey::App;
use gooey_widgets::Input;

fn main() {
    App::default().run(|cx, _window| {
        let value = cx.new_dynamic(String::from("empty string overflow"));
        Input::new(value).on_update(move |new| {
            println!("{new}");
        })
    })
}
