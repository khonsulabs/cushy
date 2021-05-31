pub use stylecs::{style_sheet, AnyStyleComponent, FallbackComponent, Style, StyleComponent};

mod alignment;
mod colors;
mod font_family;
mod font_size;
mod font_style;
mod lines;
mod surround;
mod weight;

pub use self::{
    alignment::*, colors::*, font_family::*, font_size::*, font_style::*, lines::*, surround::*,
    weight::*,
};
