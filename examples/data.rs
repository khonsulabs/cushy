use cushy::widget::{IntoWidgetList, MakeWidget};
use cushy::widgets::Data;
use cushy::Run;
use cushy::widgets::label::Displayable;

#[derive(Debug)]
enum Fruit {
    Banana,
    Apple,
}

fn data() -> impl MakeWidget {
    let label_1 = "A banana!"
        .to_label();

    let widget1 = Data::new_wrapping(Fruit::Banana,label_1)
        .into_button()
        .on_click(|event|{
            println!("Banana clicked!");
            // FIXME can the data be accessed here?
        });
    let widget2 = Data::new_wrapping(Fruit::Apple, "An apple."
        .to_label()
        .into_button()
        .on_click(|event|{
            println!("Apple clicked!");
            // FIXME what about in here?
        })
    );

    // this works, but it's not useful here.
    let data1 = widget2.data();

    widget1
        .and(widget2)
        .into_rows()


    // and it cannot be accessed here as it's been consumed.
    //let data1 = widget2.data();
}

fn main() -> cushy::Result {
    data().run()
}

#[test]
fn runs() {
    cushy::example!(data);
}
