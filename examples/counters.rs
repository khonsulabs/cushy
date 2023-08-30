use gooey::App;
use gooey_core::Children;
use gooey_widgets::{Button, Flex};

fn main() {
    App::default().run(|cx, _window| {
        let counters = cx.new_dynamic(Children::new(cx));

        Flex::rows(
            Children::new(cx)
                .with_widget(
                    Button::default()
                        .label("Create Counter")
                        .on_click(move |_| {
                            counters.map_mut({
                                |counters| {
                                    counters.push(|cx| {
                                        let counter = cx.new_dynamic(None::<usize>);
                                        let label =
                                            counter.map_each(|count| format!("{count:?}")).unwrap();
                                        Button::default().label(label).on_click(move |_| {
                                            counter.set(Some(
                                                counter.get().unwrap().unwrap_or_default() + 1,
                                            ));
                                        })
                                    })
                                }
                            });
                        }),
                )
                .with_widget(Flex::columns(counters)),
        )
    })
}
