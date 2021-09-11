use std::{any::TypeId, convert::TryFrom, ops::Deref};

use gooey_core::{
    figures::{Point, Rect, Rectlike, Size, Vector, Vectorlike},
    styles::{border::Border, BackgroundColor, Intent, Padding, Style, TabIndex},
    AnyTransmogrifier, AnyTransmogrifierContext, AnyWidget, Scaled, Transmogrifier,
    TransmogrifierContext, TransmogrifierState, Widget, WidgetId, WidgetRegistration,
};
use gooey_renderer::Renderer;
use winit::event::{ElementState, MouseButton, ScanCode, VirtualKeyCode};

use crate::Rasterizer;

pub trait WidgetRasterizer<R: Renderer>: Transmogrifier<Rasterizer<R>> + Sized + 'static {
    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>()
    }

    fn render_within(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        bounds: Rect<f32, Scaled>,
        parent_id: Option<&WidgetId>,
        parent_style: &Style,
    ) {
        if let Some(clipped) = context.frontend.clipped_to(bounds) {
            let bounds = bounds.as_sized();
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

            let location = Point::from_figures(
                border.left.map(|l| l.width).unwrap_or_default(),
                border.top.map(|l| l.width).unwrap_or_default(),
            ) + Vector::from_figures(
                padding.left.unwrap_or_default(),
                padding.top.unwrap_or_default(),
            );

            let content = (bounds.size - border.minimum_size() - padding.minimum_size())
                .max(&Size::default());

            let area = ContentArea {
                location,
                size: ContentSize {
                    content,
                    padding,
                    border,
                },
            };

            self.render_within_content_area(context, &clipped, &area, &effective_style);
            clipped.rasterized_widget(
                context.registration.id().clone(),
                area.translate(
                    clipped
                        .renderer()
                        .unwrap()
                        .clip_bounds()
                        .origin()
                        .to_vector(),
                ),
                <Self::Widget as Widget>::FOCUSABLE,
                parent_id,
                context.style.get::<TabIndex>().copied(),
                effective_style.get::<Intent>().copied(),
            );
        }
    }

    fn render_within_content_area(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        rasterizer: &Rasterizer<R>,
        area: &ContentArea,
        effective_style: &Style,
    ) {
        if let Some(&color) = <Self::Widget as Widget>::background_color(effective_style) {
            let renderer = rasterizer.renderer().unwrap();
            renderer.fill_rect_with_style::<BackgroundColor, _>(
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

        let bounds = renderer.bounds().as_sized();
        // The top and bottom borders will draw full width always
        if let Some(width) = top_width {
            renderer.fill_rect(
                &Rect::sized(bounds.origin, Size::new(bounds.size.width, width.get())),
                border
                    .top
                    .as_ref()
                    .unwrap()
                    .color
                    .themed_color(renderer.theme()),
            );
        }
        if let Some(width) = bottom_width {
            renderer.fill_rect(
                &Rect::sized(
                    Point::from_y(bounds.size.height - width.get()),
                    Size::from_figures(bounds.size.width(), width),
                ),
                border
                    .bottom
                    .as_ref()
                    .unwrap()
                    .color
                    .themed_color(renderer.theme()),
            );
        }

        // The left and right borders will shrink if top/bottom are drawn to
        // ensure no overlaps. This allows alpha borders to render properly.
        if let Some(width) = left_width {
            renderer.fill_rect(
                &Rect::sized(
                    Point::from_y(top_width.unwrap_or_default().get()),
                    Size::from_figures(
                        width,
                        bounds.size.height() - bottom_width.unwrap_or_default(),
                    ),
                ),
                border
                    .left
                    .as_ref()
                    .unwrap()
                    .color
                    .themed_color(renderer.theme()),
            );
        }

        if let Some(width) = right_width {
            renderer.fill_rect(
                &Rect::sized(
                    Point::from_figures(bounds.size.width() - width, top_width.unwrap_or_default()),
                    Size::from_figures(
                        width,
                        bounds.size.height() - bottom_width.unwrap_or_default(),
                    ),
                ),
                border
                    .left
                    .as_ref()
                    .unwrap()
                    .color
                    .themed_color(renderer.theme()),
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
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled>;

    fn content_size(
        &self,
        context: TransmogrifierContext<'_, Self, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> ContentSize {
        let effective_style = context
            .frontend
            .ui
            .stylesheet()
            .effective_style_for::<<Self as Transmogrifier<Rasterizer<R>>>::Widget>(
                context.style().clone(),
                context.ui_state,
            );
        let padding = effective_style.get_or_default::<Padding>();
        let border = effective_style.get_or_default::<Border>();
        let constraints = Size::new(
            constraints
                .width
                .map(|width| width - border.minimum_width().get() - padding.minimum_width().get()),
            constraints.height.map(|height| {
                height - border.minimum_height().get() - padding.minimum_height().get()
            }),
        );
        let mut temp_context = context.with_style(effective_style);
        ContentSize {
            content: self.measure_content(&mut temp_context, constraints),
            padding,
            border,
        }
    }

    /// Executed when this widget receives input focus.
    #[allow(unused_variables)]
    fn focused(&self, context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>) {}

    /// Executed when this widget loses input focus.
    #[allow(unused_variables)]
    fn blurred(&self, context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>) {}

    #[allow(unused_variables)]
    fn hit_test(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> bool {
        true
    }

    #[allow(unused_variables)]
    fn hovered(&self, context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>) {}

    #[allow(unused_variables)]
    fn unhovered(&self, context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>) {}

    #[allow(unused_variables)]
    fn mouse_move(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> bool {
        self.hit_test(context, location, area)
    }

    #[allow(unused_variables)]
    fn mouse_down(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> EventStatus {
        EventStatus::Ignored
    }

    #[allow(unused_variables)]
    fn mouse_drag(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) {
    }

    #[allow(unused_variables)]
    fn mouse_up(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        button: MouseButton,
        location: Option<Point<f32, Scaled>>,
        area: &ContentArea,
    ) {
    }

    #[allow(unused_variables)]
    fn keyboard(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        scancode: ScanCode,
        keycode: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> EventStatus {
        EventStatus::Ignored
    }

    #[allow(unused_variables)]
    fn receive_character(
        &self,
        context: &mut TransmogrifierContext<'_, Self, Rasterizer<R>>,
        character: char,
    ) -> EventStatus {
        EventStatus::Ignored
    }
}

pub trait AnyWidgetRasterizer<R: Renderer>: AnyTransmogrifier<Rasterizer<R>> + Send + Sync {
    fn render_within(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        bounds: Rect<f32, Scaled>,
        parent_id: Option<&WidgetId>,
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
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled>;

    fn content_size(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> ContentSize;

    fn focused(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>);

    fn blurred(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>);

    fn hit_test(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> bool;

    fn hovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>);

    fn unhovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>);

    fn mouse_move(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> bool;

    fn mouse_down(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> EventStatus;

    fn mouse_drag(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    );

    fn mouse_up(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Option<Point<f32, Scaled>>,
        area: &ContentArea,
    );
    fn keyboard(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        scancode: ScanCode,
        keycode: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> EventStatus;

    fn receive_character(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        character: char,
    ) -> EventStatus;
}

impl<T, R> AnyWidgetRasterizer<R> for T
where
    T: WidgetRasterizer<R> + AnyTransmogrifier<Rasterizer<R>> + Send + Sync + 'static,
    R: Renderer,
{
    fn render_within(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        bounds: Rect<f32, Scaled>,
        parent_id: Option<&WidgetId>,
        parent_style: &Style,
    ) {
        <Self as WidgetRasterizer<R>>::render_within(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            bounds,
            parent_id,
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
        constraints: Size<Option<f32>, Scaled>,
    ) -> Size<f32, Scaled> {
        <Self as WidgetRasterizer<R>>::measure_content(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            constraints,
        )
    }

    fn content_size(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        constraints: Size<Option<f32>, Scaled>,
    ) -> ContentSize {
        <Self as WidgetRasterizer<R>>::content_size(
            self,
            TransmogrifierContext::try_from(context).unwrap(),
            constraints,
        )
    }

    fn hit_test(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> bool {
        <Self as WidgetRasterizer<R>>::hit_test(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            location,
            area,
        )
    }

    fn hovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        <Self as WidgetRasterizer<R>>::hovered(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
        );
    }

    fn unhovered(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        <Self as WidgetRasterizer<R>>::unhovered(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
        );
    }

    fn mouse_move(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> bool {
        <Self as WidgetRasterizer<R>>::mouse_move(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            location,
            area,
        )
    }

    fn mouse_down(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) -> EventStatus {
        <Self as WidgetRasterizer<R>>::mouse_down(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            button,
            location,
            area,
        )
    }

    fn mouse_drag(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Point<f32, Scaled>,
        area: &ContentArea,
    ) {
        <Self as WidgetRasterizer<R>>::mouse_drag(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            button,
            location,
            area,
        );
    }

    fn mouse_up(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        button: MouseButton,
        location: Option<Point<f32, Scaled>>,
        area: &ContentArea,
    ) {
        <Self as WidgetRasterizer<R>>::mouse_up(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            button,
            location,
            area,
        );
    }

    fn keyboard(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        scancode: ScanCode,
        keycode: Option<VirtualKeyCode>,
        state: ElementState,
    ) -> EventStatus {
        <Self as WidgetRasterizer<R>>::keyboard(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            scancode,
            keycode,
            state,
        )
    }

    fn receive_character(
        &self,
        context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>,
        character: char,
    ) -> EventStatus {
        <Self as WidgetRasterizer<R>>::receive_character(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
            character,
        )
    }

    fn focused(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        <Self as WidgetRasterizer<R>>::focused(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
        );
    }

    fn blurred(&self, context: &mut AnyTransmogrifierContext<'_, Rasterizer<R>>) {
        <Self as WidgetRasterizer<R>>::blurred(
            self,
            &mut TransmogrifierContext::try_from(context).unwrap(),
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

#[derive(Default, Debug, Clone)]
pub struct ContentSize {
    pub content: Size<f32, Scaled>,
    pub padding: Padding,
    pub border: Border,
}

impl ContentSize {
    #[must_use]
    pub fn total_size(&self) -> Size<f32, Scaled> {
        self.content + self.padding.minimum_size() + self.border.minimum_size()
    }
}

#[derive(Default, Debug, Clone)]
#[must_use]
pub struct ContentArea {
    pub location: Point<f32, Scaled>,
    pub size: ContentSize,
}

impl ContentArea {
    pub fn sized(size: Size<f32, Scaled>) -> Self {
        Self {
            location: Point::default(),
            size: ContentSize {
                content: size,
                padding: Padding::default(),
                border: Border::default(),
            },
        }
    }

    /// Returns the bounds of the content area.
    #[must_use]
    pub fn content_bounds(&self) -> Rect<f32, Scaled> {
        Rect::sized(self.location, self.size.content)
    }

    /// Returns the entire area including padding and border.
    #[must_use]
    pub fn bounds(&self) -> Rect<f32, Scaled> {
        Rect::sized(
            self.location
                - Vector::new(
                    self.size.border.left.map_or(0., |b| b.width.get())
                        + self.size.padding.left.unwrap_or_default().get(),
                    self.size.border.top.map_or(0., |b| b.width.get())
                        + self.size.padding.top.unwrap_or_default().get(),
                ),
            self.size.content + self.size.border.minimum_size() + self.size.padding.minimum_size(),
        )
    }

    pub fn translate(&self, by: Vector<f32, Scaled>) -> Self {
        Self {
            location: self.location + by,
            size: self.size.clone(),
        }
    }
}
