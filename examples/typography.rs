use cushy::widget::MakeWidget;
use cushy::Run;

fn main() -> cushy::Result {
    "Heading 1"
        .h1()
        .and("Heading 2".h2())
        .and("Heading 3".h3())
        .and("Heading 4".h4())
        .and("Heading 5".h5())
        .and("Heading 6".h6())
        .and("Regular Text")
        .and("Small Text".small())
        .and("X-Small Text".x_small())
        .into_rows()
        .centered()
        .run()
}
