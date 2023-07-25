use gooey_widgets::Button;

fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let counter = cx.new_dynamic(0i32);
        let label = counter.map_each(|count| count.to_string()).unwrap();
        Button::new(label).on_click(move |_| {
            counter.set(counter.get().unwrap() + 1);
        })
    })
}
