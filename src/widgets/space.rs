use kludgine::figures::units::UPx;
use kludgine::figures::Size;
use kludgine::Color;

use crate::context::{GraphicsContext, LayoutContext};
use crate::value::{IntoValue, Value};
use crate::widget::Widget;
use crate::ConstraintLimit;

/// A widget that occupies space, optionally filling it with a color.
#[derive(Debug, Clone)]
pub struct Space {
    color: Value<Color>,
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
            color: Value::Constant(Color::CLEAR_BLACK),
        }
    }

    /// Returns a widget that fills its space with `color`.
    #[must_use]
    pub fn colored(color: impl IntoValue<Color>) -> Self {
        Self {
            color: color.into_value(),
        }
    }
}

impl Widget for Space {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let color = self.color.get_tracking_redraw(context);
        context.fill(color);
    }

    fn layout(
        &mut self,
        _available_space: Size<ConstraintLimit>,
        _context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        Size::default()
    }
}
