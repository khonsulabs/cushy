use kludgine::figures::units::UPx;
use kludgine::figures::Size;

use crate::context::Context;
use crate::graphics::Graphics;
use crate::styles::Styles;
use crate::widget::{BoxedWidget, ManagedWidget, Widget};
use crate::ConstraintLimit;

#[derive(Debug)]
pub struct Style {
    styles: Styles,
    child: BoxedWidget,
    mounted_child: Option<ManagedWidget>,
}

impl Style {
    pub fn new<W>(styles: Styles, child: W) -> Self
    where
        W: Widget,
    {
        Self {
            styles,
            child: BoxedWidget::new(child),
            mounted_child: None,
        }
    }
}

impl Widget for Style {
    fn mounted(&mut self, context: &mut Context<'_, '_>) {
        context.attach_styles(self.styles.clone());
        self.mounted_child = Some(context.push_child(self.child.clone()));
    }

    fn unmounted(&mut self, context: &mut Context<'_, '_>) {
        let child = self
            .mounted_child
            .take()
            .expect("unmounted without being mounted");
        context.remove_child(&child);
    }

    fn redraw(&mut self, graphics: &mut Graphics<'_, '_, '_>, context: &mut Context<'_, '_>) {
        context
            .for_other(
                self.mounted_child
                    .as_ref()
                    .expect("measuring without being mounted"),
            )
            .redraw(graphics);
    }

    fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        graphics: &mut Graphics<'_, '_, '_>,
        context: &mut Context<'_, '_>,
    ) -> Size<UPx> {
        context
            .for_other(
                self.mounted_child
                    .as_ref()
                    .expect("measuring without being mounted"),
            )
            .measure(available_space, graphics)
    }
}
