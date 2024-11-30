use cushy::context::{GraphicsContext, LayoutContext, Trackable};
use cushy::figures::units::{Px, UPx};
use cushy::figures::{IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero};
use cushy::kludgine::text::{MeasuredText, TextOrigin};
use cushy::styles::components::IntrinsicPadding;
use cushy::value::{Dynamic, IntoValue, Value};
use cushy::widget::{WidgetLayout, WrappedLayout, WrapperWidget};
use cushy::widgets::input::InputValue;
use cushy::ConstraintLimit;

fn composition_wrapperwidget() -> impl cushy::widget::MakeWidget {
    // ANCHOR: definition
    use cushy::widget::{MakeWidget, WidgetRef};

    #[derive(Debug)]
    struct FormField {
        label: Value<String>,
        field: WidgetRef,
    }

    impl FormField {
        pub fn new(label: impl IntoValue<String>, field: impl MakeWidget) -> Self {
            Self {
                label: label.into_value(),
                field: WidgetRef::new(field),
            }
        }
    }
    // ANCHOR_END: definition

    // ANCHOR: helpers
    impl FormField {
        fn measured_label(
            &self,
            context: &mut GraphicsContext<'_, '_, '_, '_>,
        ) -> MeasuredText<Px> {
            self.label.invalidate_when_changed(context);
            self.label.map(|label| context.gfx.measure_text(label))
        }

        fn label_and_padding_size(
            &self,
            context: &mut GraphicsContext<'_, '_, '_, '_>,
        ) -> Size<UPx> {
            let label_size = self.measured_label(context).size.into_unsigned();
            let padding = context.get(&IntrinsicPadding).into_upx(context.gfx.scale());
            Size::new(label_size.width, label_size.height + padding)
        }
    }
    // ANCHOR_END: helpers

    // ANCHOR: wrapperwidget-a
    impl WrapperWidget for FormField {
        fn child_mut(&mut self) -> &mut WidgetRef {
            &mut self.field
        }

        // ANCHOR_END: wrapperwidget-a

        // ANCHOR: wrapperwidget-b
        fn adjust_child_constraints(
            &mut self,
            available_space: Size<ConstraintLimit>,
            context: &mut LayoutContext<'_, '_, '_, '_>,
        ) -> Size<ConstraintLimit> {
            let label_and_padding = self.label_and_padding_size(context);
            Size::new(
                available_space.width,
                available_space.height - label_and_padding.height,
            )
        }

        // ANCHOR_END: wrapperwidget-b

        // ANCHOR: wrapperwidget-c
        fn position_child(
            &mut self,
            child_layout: WidgetLayout,
            available_space: Size<ConstraintLimit>,
            context: &mut LayoutContext<'_, '_, '_, '_>,
        ) -> WrappedLayout {
            let label_and_padding = self.label_and_padding_size(context).into_signed();
            let full_size = Size::new(
                available_space
                    .width
                    .min()
                    .max(child_layout.size.width)
                    .into_signed()
                    .max(label_and_padding.width),
                child_layout.size.height.into_signed() + label_and_padding.height,
            );
            WrappedLayout {
                child: Rect::new(
                    Point::new(Px::ZERO, label_and_padding.height),
                    Size::new(full_size.width, child_layout.size.height.into_signed()),
                ),
                size: full_size.into_unsigned(),
                baseline: child_layout.baseline,
            }
        }

        // ANCHOR_END: wrapperwidget-c

        // ANCHOR: wrapperwidget-d
        fn redraw_foreground(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
            let label = self.measured_label(context);
            context.gfx.draw_measured_text(&label, TextOrigin::TopLeft);
        }
    }
    // ANCHOR_END: wrapperwidget-d

    FormField::new(
        "Label",
        Dynamic::<String>::default()
            .into_input()
            .placeholder("Field"),
    )
}

fn main() {
    cushy::example!(composition_wrapperwidget).untested_still_frame();
}

#[test]
fn runs() {
    main();
}
