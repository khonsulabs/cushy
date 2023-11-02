use kludgine::figures::units::UPx;
use kludgine::figures::Size;

use crate::context::{AsEventContext, EventContext, GraphicsContext};
use crate::styles::Styles;
use crate::widget::{ChildWidget, MakeWidget, Widget};
use crate::ConstraintLimit;

/// A widget that applies a set of [`Styles`] to all contained widgets.
#[derive(Debug)]
pub struct Style {
    styles: Styles,
    child: ChildWidget,
}

impl Style {
    /// Returns a new widget that applies `styles` to `child` and any children
    /// it may have.
    pub fn new(styles: impl Into<Styles>, child: impl MakeWidget) -> Self {
        Self {
            styles: styles.into(),
            child: ChildWidget::new(child),
        }
    }
}

impl Widget for Style {
    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {
        context.attach_styles(self.styles.clone());
    }

    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let child = self.child.mounted(&mut context.as_event_context());
        context.for_other(&child).redraw();
    }

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let child = self.child.mounted(&mut context.as_event_context());
        context.for_other(&child).measure(available_space)
    }
}
