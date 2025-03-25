use figures::Size;
use kludgine::Color;

use crate::context::{GraphicsContext, LayoutContext};
use crate::reactive::value::{IntoValue, Value};
use crate::styles::components::PrimaryColor;
use crate::styles::{DynamicComponent, IntoDynamicComponentValue};
use crate::widget::{Widget, WidgetLayout};
use crate::ConstraintLimit;

/// A widget that occupies space, optionally filling it with a color.
#[derive(Debug, Clone)]
pub struct Space {
    color: Value<ColorSource>,
}

impl Default for Space {
    fn default() -> Self {
        Self::clear()
    }
}

impl Space {
    /// Returns a widget that draws nothing.
    #[must_use]
    pub const fn clear() -> Self {
        Self {
            color: Value::Constant(ColorSource::Color(Color::CLEAR_BLACK)),
        }
    }

    /// Returns a widget that fills its space with `color`.
    #[must_use]
    pub fn colored(color: impl IntoValue<Color>) -> Self {
        Self {
            color: color
                .into_value()
                .map_each(|color| ColorSource::Color(*color)),
        }
    }

    /// Returns a spacer that fills itself with `dynamic`'s color.
    pub fn dynamic(dynamic: impl IntoDynamicComponentValue) -> Self {
        Self {
            color: dynamic
                .into_dynamic_component()
                .map_each(|component| ColorSource::Dynamic(component.clone())),
        }
    }

    /// Returns a spacer that fills itself with the value of [`PrimaryColor`].
    #[must_use]
    pub fn primary() -> Self {
        Self::dynamic(PrimaryColor)
    }
}

impl Widget for Space {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let source = self.color.get_tracking_redraw(context);
        let color = match source {
            ColorSource::Color(color) => color,
            ColorSource::Dynamic(component) => component
                .resolve(context)
                .and_then(|component| Color::try_from(component).ok())
                .unwrap_or(Color::CLEAR_BLACK),
        };
        context.fill(color);
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        _context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        available_space.map(ConstraintLimit::min).into()
    }
}

#[derive(Debug, PartialEq, Clone)]
enum ColorSource {
    Color(Color),
    Dynamic(DynamicComponent),
}
