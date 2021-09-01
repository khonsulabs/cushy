pub use stylecs::{AnyStyleComponent, FallbackComponent, Style, StyleComponent};

mod alignment;
/// Types for adding a border to a widget.
pub mod border;
mod colors;
mod focus;
mod font_family;
mod font_size;
mod font_style;
mod intent;
mod lines;
/// Types for adding padding to a widget.
pub mod padding;
/// Types for defining sets of rules.
pub mod style_sheet;
mod surround;
mod weight;

pub use self::{
    alignment::{Alignment, VerticalAlignment},
    border::{Border, BorderOptions},
    colors::{
        BackgroundColor, Color, ColorPair, ForegroundColor, HighlightColor, SystemTheme, TextColor,
    },
    focus::{Autofocus, TabIndex},
    font_family::FontFamily,
    font_size::FontSize,
    font_style::FontStyle,
    intent::Intent,
    lines::LineWidth,
    padding::Padding,
    surround::Surround,
    weight::Weight,
};
