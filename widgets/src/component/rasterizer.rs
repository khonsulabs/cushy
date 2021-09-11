use gooey_core::{figures::Size, Scaled, Transmogrifier, TransmogrifierContext, Widget, WidgetRef};
use gooey_rasterizer::{ContentArea, Rasterizer, Renderer, WidgetRasterizer};

use super::Component;
use crate::component::{Behavior, ComponentTransmogrifier};

impl<R: Renderer, B: Behavior> WidgetRasterizer<R> for ComponentTransmogrifier<B> {
    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    ) {
        context.frontend.with_transmogrifier(
            context.widget.content.id(),
            |child_transmogrifier, mut child_context| {
                child_transmogrifier.render_within(
                    &mut child_context,
                    content_area.content_bounds(),
                    Some(context.registration.id()),
                    context.style(),
                );
            },
        );
    }

    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        context
            .frontend
            .with_transmogrifier(
                context.widget.content.id(),
                |child_transmogrifier, mut child_context| {
                    child_transmogrifier
                        .content_size(&mut child_context, constraints)
                        .total_size()
                },
            )
            .unwrap_or_default()
    }
}

impl<B: Behavior, R: Renderer> From<ComponentTransmogrifier<B>>
    for gooey_rasterizer::RegisteredTransmogrifier<R>
{
    fn from(transmogrifier: ComponentTransmogrifier<B>) -> Self {
        Self(std::boxed::Box::new(transmogrifier))
    }
}

impl<B: Behavior, R: Renderer> Transmogrifier<Rasterizer<R>> for ComponentTransmogrifier<B> {
    type State = ();
    type Widget = Component<B>;

    fn initialize(
        &self,
        component: &mut Self::Widget,
        widget: &WidgetRef<Self::Widget>,
        frontend: &Rasterizer<R>,
    ) -> Self::State {
        Self::initialize_component(component, widget, frontend);
    }

    fn receive_command(
        &self,
        command: <Self::Widget as Widget>::Command,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
    ) {
        Self::forward_command_to_content(command, context);
    }
}
