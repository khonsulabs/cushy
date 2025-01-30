use cushy::reactive::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::input::InputValue;
use cushy::Run;
use figures::units::Lp;

fn main() -> cushy::Result {
    let celsius = Dynamic::new(100f32);
    let farenheit = celsius.linked(
        |celsius| *celsius * 9. / 5. + 32.,
        |farenheit| (*farenheit - 32.) * 5. / 9.,
    );

    let celsius_string = celsius.linked_string();
    let farenheight_string = farenheit.linked_string();

    celsius_string
        .into_input()
        .expand()
        .and("Celsius =")
        .and(farenheight_string.into_input().expand())
        .and("Farenheit")
        .into_columns()
        .pad()
        .width(Lp::inches(4))
        .into_window()
        .titled("Temperature Converter")
        .resize_to_fit(true)
        .run()
}
