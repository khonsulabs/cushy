use cushy::widget::MakeWidget;
use cushy::widgets::Disclose;
use cushy::Run;

fn main() -> cushy::Result {
    Disclose::new(
        "This is some inner content"
            .align_left()
            .and(Disclose::new("This is even further inside".contain()))
            .into_rows(),
    )
    .labelled_by("This demonstrates the Disclose widget")
    .run()
}
