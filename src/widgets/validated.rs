//! A widget that displays the result of validation.

use std::fmt::Debug;

use kludgine::Color;
use crate::MaybeLocalized;
use crate::styles::components::{
    ErrorColor, LineHeight, LineHeight2, OutlineColor, TextColor, TextSize, TextSize2,
};
use crate::styles::Dimension;
use crate::reactive::value::{
    Destination, Dynamic, IntoDynamic, IntoValue, MapEach, Source, Validation, Value,
};
use crate::widget::{MakeWidget, MakeWidgetWithTag, WidgetInstance, WidgetRef, WidgetTag, WrapperWidget};
use crate::widgets::label::Displayable;

/// A widget that displays validation information around another widget.
///
/// This widget overrides the outline color of its child to be the theme's error
/// color.
///
/// Additionally, a message may be shown below the content widget. If there is a
/// validation error, it is shown. Otherwise, an optional hint message is
/// supported.
#[derive(Debug)]
pub struct Validated {
    hint: Value<MaybeLocalized>,
    validation: Dynamic<Validation>,
    validated: WidgetInstance,
}

impl Validated {
    /// Returns a widget that displays validation information around `validated`
    /// based on `validation`.
    #[must_use]
    pub fn new(validation: impl IntoDynamic<Validation>, validated: impl MakeWidget) -> Self {
        Self {
            validation: validation.into_dynamic(),
            validated: validated.make_widget(),
            hint: Value::default(),
        }
    }

    /// Sets the hint message to be displayed when there is no validation error.
    #[must_use]
    pub fn hint(mut self, hint: impl IntoValue<MaybeLocalized>) -> Self {
        self.hint = hint.into_value();
        self
    }
}

impl MakeWidgetWithTag for Validated {
    fn make_with_tag(self, id: WidgetTag) -> WidgetInstance {
        let message: Dynamic<MaybeLocalized> = self.validation.map_each_cloned(move |validation| validation.message(&self.hint).get());

        let error_color = Dynamic::new(Color::CLEAR_BLACK);
        let default_color = Dynamic::new(Color::CLEAR_BLACK);
        let color = (&self.validation, &error_color, &default_color).map_each(
            |(validation, error, default)| {
                if validation.is_error() {
                    *error
                } else {
                    *default
                }
            },
        );

        ValidatedWidget {
            contents: WidgetRef::new(
                self.validated
                    .with(&OutlineColor, color.clone())
                    .and(
                        message.map_each_cloned({
                            move |message|{
                                let label = match message {
                                    MaybeLocalized::Localized(localized) => {
                                        let (tag, _id) = WidgetTag::new();
                                        localized.make_with_tag(tag)
                                    },
                                    MaybeLocalized::Text(meh) => meh.into_label().make_widget(),
                                };

                                label
                            }
                        })
                            .with(&TextColor, color)
                            .with_dynamic(&TextSize, ValidatedTextSize)
                            .with_dynamic(&LineHeight, ValidatedLineHeight)
                            .align_left()
                    )
                    .into_rows(),
            ),
            error_color,
            default_color,
        }
        .make_with_tag(id)
    }
}

#[derive(Debug)]
struct ValidatedWidget {
    contents: WidgetRef,
    error_color: Dynamic<Color>,
    default_color: Dynamic<Color>,
}

impl WrapperWidget for ValidatedWidget {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.contents
    }

    fn redraw_background(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        self.error_color.set(context.get(&InvalidTextColor));
        self.default_color.set(context.get(&HintTextColor));
    }
}

define_components! {
    Validated {
        /// The color of the hint text.
        HintTextColor(Color, "hint_color", @OutlineColor)
        /// The color of invalid text.
        InvalidTextColor(Color, "invalid_color", @ErrorColor)
        /// The text size for the validation message in a [`Validated`] widget.
        ValidatedTextSize(Dimension, "text_size", @TextSize2)
        /// The line hgiht for the validation message in a [`Validated`] widget.
        ValidatedLineHeight(Dimension, "line_height", @LineHeight2)
    }
}
