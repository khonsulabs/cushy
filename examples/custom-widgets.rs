//! This example shows two approaches to writing custom widgets: implementing
//! traits or using the [`Custom`] widget with callbacks.

use gooey::value::Dynamic;
use gooey::widget::{MakeWidget, MakeWidgetWithId, Widget, WidgetInstance, WidgetTag, HANDLED};
use gooey::widgets::Custom;
use gooey::Run;
use kludgine::figures::units::{Lp, UPx};
use kludgine::figures::{ScreenScale, Size};
use kludgine::Color;

fn main() -> gooey::Result {
    "Inline Widgets"
        .and(callback_widget())
        .into_rows()
        .and(
            "impl MakeWidget"
                .and(ToggleMakeWidget::default())
                .into_rows(),
        )
        .and("impl Widget".and(impl_widget()).into_rows())
        .into_columns()
        .centered()
        .run()
}

/// This function returns a [`Custom`] widget that implements its functionality
/// using callbacks.
///
/// This approach was added to make it easy to create one-off widgets in a
/// hierarchy to handle events or other purpose-built functions.
fn callback_widget() -> impl MakeWidgetWithId {
    // This implementation and the impl `Widget` implementation both use the
    // same Dynamic value setup.
    let toggle = Toggle::default();

    Custom::empty()
        .background_color(toggle.color)
        .on_hit_test(|_, _| true)
        .on_mouse_down(move |_, _, _, _| {
            toggle.value.toggle();
            HANDLED
        })
        .height(Lp::inches(1))
}

/// A second approach is to implement [`MakeWidgetWithId`] for a type. This
/// allows any type to be used when composing your UI that know how to create a
/// widget.
///
/// This enables using callback-based widgets (or any other combination of
/// widgets) in a reusable fashion.
///
/// [`MakeWidget`] is implemented automatically for all types that implement
/// [`MakeWidgetWithId`]. The difference between the traits is purely whether
/// allowing a caller instantiating your custom widget to provide an id for the
/// widget. These IDs are used when configuring custom tab orders, so if your
/// widget or any of its children aren't focusable, implementing [`MakeWidget`]
/// directly will make more sense.
#[derive(Default)]
struct ToggleMakeWidget(Toggle);

impl MakeWidgetWithId for ToggleMakeWidget {
    fn make_with_id(self, id: WidgetTag) -> WidgetInstance {
        // In a real code base, the contents of callback_widget() would go here
        callback_widget().make_with_id(id)
    }
}

/// This function returns [`Toggle`] using its [`Widget`] implementation.
///
/// This is the lowest-level way to implement a Widget, but it also provides the
/// most power and flexibility.
fn impl_widget() -> impl MakeWidgetWithId {
    Toggle::default()
}

#[derive(Debug)]
struct Toggle {
    value: Dynamic<bool>,
    color: Dynamic<Color>,
}

impl Default for Toggle {
    fn default() -> Self {
        let value = Dynamic::default();
        let color = value.map_each(|on| if *on { Color::RED } else { Color::BLUE });
        Self { value, color }
    }
}

impl Widget for Toggle {
    fn redraw(&mut self, context: &mut gooey::context::GraphicsContext<'_, '_, '_, '_, '_>) {
        context.fill(self.color.get_tracking_refresh(context));
    }

    fn layout(
        &mut self,
        available_space: Size<gooey::ConstraintLimit>,
        context: &mut gooey::context::LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        Size::new(
            available_space.width.min(),
            Lp::inches(1).into_upx(context.gfx.scale()),
        )
    }

    fn hit_test(
        &mut self,
        _location: kludgine::figures::Point<kludgine::figures::units::Px>,
        _context: &mut gooey::context::EventContext<'_, '_>,
    ) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        _location: kludgine::figures::Point<kludgine::figures::units::Px>,
        _device_id: kludgine::app::winit::event::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        _context: &mut gooey::context::EventContext<'_, '_>,
    ) -> gooey::widget::EventHandling {
        self.value.toggle();

        HANDLED
    }
}
