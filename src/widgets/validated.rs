use std::fmt::Debug;

use kludgine::figures::units::Lp;
use kludgine::Color;

use crate::styles::components::{LineHeight, OutlineColor, TextColor, TextSize};
use crate::value::{Dynamic, IntoDynamic, IntoValue, MapEach, Validation, Value};
use crate::widget::{MakeWidget, WidgetInstance, WidgetRef, WrapperWidget};

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
    hint: Value<String>,
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
    pub fn hint(mut self, hint: impl IntoValue<String>) -> Self {
        self.hint = hint.into_value();
        self
    }
}

impl MakeWidget for Validated {
    fn make_widget(self) -> WidgetInstance {
        let message = match self.hint {
            Value::Constant(hint) => self
                .validation
                .map_each(move |validation| validation.message(&hint).to_string()),
            Value::Dynamic(hint) => (&hint, &self.validation)
                .map_each(move |(hint, validation)| validation.message(hint).to_string()),
        };

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
                        message
                            .with(&TextColor, color)
                            // TODO these should be components
                            .with(&TextSize, Lp::points(9))
                            .with(&LineHeight, Lp::points(13))
                            .align_left(),
                    )
                    .into_rows(),
            ),
            error_color,
            default_color,
        }
        .make_widget()
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

    fn redraw_background(
        &mut self,
        context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>,
    ) {
        // TODO move these to components.
        self.error_color.set(context.theme().error.color);
        self.default_color.set(context.theme().surface.outline);
    }
}
