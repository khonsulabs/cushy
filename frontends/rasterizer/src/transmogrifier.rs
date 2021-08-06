use std::{any::TypeId, convert::TryFrom, ops::Deref};

use gooey_core::{
    euclid::{Point2D, Rect, Size2D},
    styles::{border::Border, BackgroundColor, Padding, Style},
    AnyTransmogrifier, AnyTransmogrifierContext, AnyWidget, Points, Transmogrifier,
    TransmogrifierContext, TransmogrifierState, Widget, WidgetRegistration,
};
use gooey_renderer::Renderer;
use winit::event::MouseButton;

use crate::Rasterizer;

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> + Sized + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>()
    }

    fn render_within(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        bounds: Rect<f32, Points>,
        parent_style: &Style,
    ) {
        if let Some(rasterizer) = context.frontend.clipped_to(bounds) {
            let effective_style = context
                .frontend
                .ui
                .stylesheet()
                .effective_style_for::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>(
                    context.style.merge_with(parent_style, true),
                    context.ui_state,
                );
            let border = effective_style.get_or_default::<Border>();
            let padding = effective_style.get_or_default::<Padding>();

            let content = (bounds.size - border.minimum_size() - padding.minimum_size())
                .max(Size2D::default());
            let remaining_width = bounds.size - content;
            // TODO support Alignment and VerticalAlignment
            let location = (remaining_width.to_vector() / 2.).to_point();

            let area = ContentArea {
                location,
                size: ContentSize {
                    content,
                    padding,
                    border,
                },
            };
            self.render_within_content_area(context, &rasterizer, &area, &effective_style);
        }
    }

    fn render_within_content_area(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        rasterizer: &Rasterizer<R>,
        area: &ContentArea,
        effective_style: &Style,
    ) {
        rasterizer.rasterized_widget(
            context.registration.id().clone(),
            rasterizer.renderer().unwrap().clip_bounds(),
        );

        if let Some(&color) = <Self::Widget as Widget>::background_color(effective_style) {
            let renderer = rasterizer.renderer().unwrap();
            renderer.fill_rect_with_style::<BackgroundColor>(
                &renderer.bounds(),
                &Style::default().with(BackgroundColor(color)),
            );
        }

        let mut context = TransmogrifierContext::new(
            context.registration.clone(),
            context.state,
            rasterizer,
            context.widget,
            context.channels,
            effective_style,
            context.ui_state,
        );

        self.render_border(rasterizer.renderer().unwrap(), &area.size.border);

        self.render(&mut context, area);
    }

    fn render_border(&self, renderer: &R, border: &Border) {
        let left_width = border
            .left
            .as_ref()
            .map(|o| o.width)
            .filter(|w| w.get() > 0.);
        let right_width = border
            .right
            .as_ref()
            .map(|o| o.width)
            .filter(|w| w.get() > 0.);
        let top_width = border
            .top
            .as_ref()
            .map(|o| o.width)
            .filter(|w| w.get() > 0.);
        let bottom_width = border
            .bottom
            .as_ref()
            .map(|o| o.width)
            .filter(|w| w.get() > 0.);

        let bounds = renderer.bounds();
        // The top and bottom borders will draw full width always
        if let Some(width) = top_width {
            renderer.fill_rect(
                &Rect::new(bounds.origin, Size2D::new(bounds.size.width, width.get())),
                border.top.as_ref().unwrap().color,
            );
        }
        if let Some(width) = bottom_width {
            renderer.fill_rect(
                &Rect::new(
                    Point2D::new(0., bounds.size.height - width.get()),
                    Size2D::new(bounds.size.width, width.get()),
                ),
                border.bottom.as_ref().unwrap().color,
            );
        }

        // The left and right borders will shrink if top/bottom are drawn to
        // ensure no overlaps. This allows alpha borders to render properly.
        if let Some(width) = left_width {
            renderer.fill_rect(
                &Rect::new(
                    Point2D::new(0., top_width.unwrap_or_default().get()),
                    Size2D::new(
                        width.get(),
                        bounds.size.height - bottom_width.unwrap_or_default().get(),
                    ),
                ),
                border.left.as_ref().unwrap().color,
            );
        }

        if let Some(width) = right_width {
            renderer.fill_rect(
                &Rect::new(
                    Point2D::new(
                        bounds.size.width - width.get(),
                        top_width.unwrap_or_default().get(),
                    ),
                    Size2D::new(
                        width.get(),
                        bounds.size.height - bottom_width.unwrap_or_default().get(),
                    ),
                ),
                border.left.as_ref().unwrap().color,
            );
        }
    }

    fn render(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        content_area: &ContentArea,
    );

    /// Calculate the content-size needed for this `widget`, trying to stay
    /// within `constraints`.
    fn measure_content(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;

    fn content_size(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> ContentSize {
        let effective_style = context
            .frontend
            .ui
            .stylesheet()
            .effective_style_for::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>(
                context.style.clone(),
                context.ui_state,
            );
        let padding = effective_style.get_or_default::<Padding>();
        let border = effective_style.get_or_default::<Border>();
        let constraints = Size2D::new(
            constraints
                .width
                .map(|width| width - border.minimum_width().get() - padding.minimum_width().get()),
            constraints.height.map(|height| {
                height - border.minimum_height().get() - padding.minimum_height().get()
            }),
        );
        ContentSize {
            content: self.measure_content(context, constraints),
            padding,
            border,
        }
    }

    #[allow(unused_variables)]
    fn hit_test(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn hovered(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {}

    #[allow(unused_variables)]
    fn unhovered(&self, context: TransmogrifierContext<'_, Self, Rasterizer<R>>) {}

    #[allow(unused_variables)]
    fn mouse_move(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> bool {
        self.hit_test(context, location, rastered_size)
    }

    #[allow(unused_variables)]
    fn mouse_down(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> EventStatus {
        EventStatus::Ignored
    }

    #[allow(unused_variables)]
    fn mouse_drag(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) {
    }

    #[allow(unused_variables)]
    fn mouse_up(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Option<Point2D<f32, Points>>,
        rastered_size: Size2D<f32, Points>,
    ) {
    }
}

pub trait AnyWidgetRasterizer<R: Renderer>: AnyTransmogrifier<Rasterizer<R>> + Send + Sync {
    fn render_within(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        bounds: Rect<f32, Points>,
        parent_style: &Style,
    );

    fn render_within_content_area(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        rasterizer: &Rasterizer<R>,
        area: &ContentArea,
        effective_style: &Style,
    );

    fn measure_content(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points>;

    fn content_size(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> ContentSize;

    fn hit_test(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> bool;

    fn hovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>);

    fn unhovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>);

    fn mouse_move(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> bool;

    fn mouse_down(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> EventStatus;

    fn mouse_drag(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    );

    fn mouse_up(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Option<Point2D<f32, Points>>,
        rastered_size: Size2D<f32, Points>,
    );
}

impl<T, R> AnyWidgetRasterizer<R> for T
where
    T: WidgetRasterizer<R> + AnyTransmogrifier<Rasterizer<R>> + Send + Sync + 'static,
    R: Renderer,
{
    fn render_within(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        bounds: Rect<f32, Points>,
        parent_style: &Style,
    ) {
        <Self as WidgetRasterizer<R>>::render_within(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            bounds,
            parent_style,
        );
    }

    fn render_within_content_area(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        rasterizer: &Rasterizer<R>,
        area: &ContentArea,
        effective_style: &Style,
    ) {
        <Self as WidgetRasterizer<R>>::render_within_content_area(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            rasterizer,
            area,
            effective_style,
        );
    }

    fn measure_content(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> Size2D<f32, Points> {
        <Self as WidgetRasterizer<R>>::measure_content(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            constraints,
        )
    }

    fn content_size(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        constraints: Size2D<Option<f32>, Points>,
    ) -> ContentSize {
        <Self as WidgetRasterizer<R>>::content_size(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            constraints,
        )
    }

    fn hit_test(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> bool {
        <Self as WidgetRasterizer<R>>::hit_test(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            location,
            rastered_size,
        )
    }

    fn hovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        <Self as WidgetRasterizer<R>>::hovered(
            self,
            TransmogrifierContext::try_from(context).unwrap(),
        );
    }

    fn unhovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        <Self as WidgetRasterizer<R>>::unhovered(
            self,
            TransmogrifierContext::try_from(context).unwrap(),
        );
    }

    fn mouse_move(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> bool {
        <Self as WidgetRasterizer<R>>::mouse_move(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            location,
            rastered_size,
        )
    }

    fn mouse_down(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) -> EventStatus {
        <Self as WidgetRasterizer<R>>::mouse_down(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            button,
            location,
            rastered_size,
        )
    }

    fn mouse_drag(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point2D<f32, Points>,
        rastered_size: Size2D<f32, Points>,
    ) {
        <Self as WidgetRasterizer<R>>::mouse_drag(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            button,
            location,
            rastered_size,
        );
    }

    fn mouse_up(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Option<Point2D<f32, Points>>,
        rastered_size: Size2D<f32, Points>,
    ) {
        <Self as WidgetRasterizer<R>>::mouse_up(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            button,
            location,
            rastered_size,
        );
    }
}

impl<R: Renderer> AnyTransmogrifier<Rasterizer<R>> for RegisteredTransmogrifier<R> {
    fn process_messages(&self, context: AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        self.0.as_ref().process_messages(context);
    }

    fn widget_type_id(&self) -> TypeId {
        self.0.widget_type_id()
    }

    fn default_state_for(
        &self,
        widget: &mut dyn AnyWidget,
        registration: &WidgetRegistration,
        frontend: &Rasterizer<R>,
    ) -> TransmogrifierState {
        self.0.default_state_for(widget, registration, frontend)
    }
}

#[derive(Debug)]
pub struct RegisteredTransmogrifier<R: Renderer>(pub Box<dyn AnyWidgetRasterizer<R>>);

impl<R: Renderer> Deref for RegisteredTransmogrifier<R> {
    type Target = Box<dyn AnyWidgetRasterizer<R>>;

    fn deref(&self) -> &'_ Self::Target {
        &self.0
    }
}

#[macro_export]
macro_rules! make_rasterized {
    ($transmogrifier:ident) => {
        impl<R: $crate::Renderer> From<$transmogrifier> for $crate::RegisteredTransmogrifier<R> {
            fn from(transmogrifier: $transmogrifier) -> Self {
                Self(std::boxed::Box::new(transmogrifier))
            }
        }
    };
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum EventStatus {
    Ignored,
    Processed,
}

#[derive(Debug, Clone)]
pub struct ContentSize {
    pub content: Size2D<f32, Points>,
    pub padding: Padding,
    pub border: Border,
}

impl ContentSize {
    #[must_use]
    pub fn total_size(&self) -> Size2D<f32, Points> {
        self.content + self.padding.minimum_size() + self.border.minimum_size()
    }
}

#[derive(Debug)]
#[must_use]
pub struct ContentArea {
    pub location: Point2D<f32, Points>,
    pub size: ContentSize,
}

impl ContentArea {
    pub fn sized(size: Size2D<f32, Points>) -> Self {
        Self {
            location: Point2D::default(),
            size: ContentSize {
                content: size,
                padding: Padding::default(),
                border: Border::default(),
            },
        }
    }

    #[must_use]
    pub fn bounds(&self) -> Rect<f32, Points> {
        Rect::new(self.location, self.size.content)
    }
}
