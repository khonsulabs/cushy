pub use stylecs::{
    palette::Srgba, AnyStyleComponent, ColorPair, Style, StyleComponent, Surround, SystemTheme,
};

mod alignment;
mod colors;
mod font_family;
mod font_size;
mod font_style;
mod lines;
mod weight;
mod widget;

pub use self::{
    alignment::*, colors::*, font_family::*, font_size::*, font_style::*, lines::*, weight::*,
    widget::*,
};
