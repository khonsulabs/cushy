use gooey_core::{
    euclid::Size2D, renderer::Renderer, Points, Transmogrifier, TransmogrifierContext, Widget,
    WidgetRef,
};
use gooey_rasterizer::{Rasterizer, WidgetRasterizer};

use super::Component;
use crate::component::{Behavior, ComponentTransmogrifier};

impl<R: Renderer, B: Behavior> WidgetRasterizer<R> for ComponentTransmogrifier<B> {
    fn render(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {
        context.frontend.with_transmogrifier(
            context.widget.content.id(),
            |child_transmogrifier, mut child_context| {
                let bounds = context
                    .frontend
                    .renderer()
                    .map(|r| r.bounds())
                    .unwrap_or_default();
                child_transmogrifier.render_within(&mut child_context, bounds, context.style);
            },
        );
    }

    fn content_size(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        context
            .frontend
            .with_transmogrifier(
                context.widget.content.id(),
                |child_transmogrifier, mut child_context| {
                    child_transmogrifier.content_size(&mut child_context, constraints)
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
