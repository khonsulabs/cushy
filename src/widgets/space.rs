use kludgine::figures::units::UPx;
use kludgine::figures::Size;

use crate::context::{GraphicsContext, LayoutContext};
use crate::widget::Widget;
use crate::ConstraintLimit;

/// A widget that does nothing and draws nothing.
#[derive(Debug, Clone)]
pub struct Space;

impl Widget for Space {
    fn redraw(&mut self, _context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {}

    fn layout(
        &mut self,
        _available_space: Size<ConstraintLimit>,
        _context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        Size::default()
    }
}
