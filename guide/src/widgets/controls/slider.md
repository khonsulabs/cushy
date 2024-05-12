# Slider

The [`Slider`][Slider] widget allows selecting one or two values between a
minimum and a maximum. This is implemented using the trait
[`SliderValue`][SliderValue], which is automatically implemented for types that
implement [`Ranged`][Ranged] and [`PercentBetween`][PercentBetween]. This
includes all numeric types in Rust.

The `Slider` widget can set either a single value or a tuple of 2 elements. When
a two element tuple is used, the slider highlights the area between the two
selected values.

[Slider]: <{{ docs }}/widgets/slider/struct.Slider.html>
[SliderValue]: <{{ docs }}/widgets/slider/trait.SliderValue.html>
[PercentBetween]: <{{ docs }}/animation/trait.PercentBetween.html>
[Ranged]: <https://docs.rs/figures/latest/figures/trait.Ranged.html>
