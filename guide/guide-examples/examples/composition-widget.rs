use cushy::context::{GraphicsContext, LayoutContext, Trackable};
use cushy::figures::units::{Px, UPx};
use cushy::figures::{IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero};
use cushy::kludgine::text::{MeasuredText, TextOrigin};
use cushy::styles::components::IntrinsicPadding;
use cushy::value::{Dynamic, IntoValue, Value};
use cushy::widget::Widget;
use cushy::widgets::input::InputValue;
use cushy::ConstraintLimit;

fn composition_widget() -> impl cushy::widget::MakeWidget {
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

    // ANCHOR: widget-a
    impl Widget for FormField {
        fn layout(
            &mut self,
            available_space: Size<ConstraintLimit>,
            context: &mut LayoutContext<'_, '_, '_, '_>,
        ) -> Size<UPx> {
            let label_and_padding = self.label_and_padding_size(context);
            let field_available_space = Size::new(
                available_space.width,
                available_space.height - label_and_padding.height,
            );
            let field = self.field.mounted(context);
            let field_size = context.for_other(&field).layout(field_available_space);

            let full_size = Size::new(
                available_space
                    .width
                    .min()
                    .max(field_size.width)
                    .max(label_and_padding.width),
                field_size.height + label_and_padding.height,
            );

            context.set_child_layout(
                &field,
                Rect::new(
                    Point::new(UPx::ZERO, label_and_padding.height),
                    Size::new(full_size.width, field_size.height),
                )
                .into_signed(),
            );

            full_size
        }

        // ANCHOR_END: widget-a

        // ANCHOR: widget-b
        fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
            let label = self.measured_label(context);
            context.gfx.draw_measured_text(&label, TextOrigin::TopLeft);

            let field = self.field.mounted(context);
            context.for_other(&field).redraw();
        }

        // ANCHOR_END: widget-b

        // ANCHOR: widget-c
        fn unmounted(&mut self, context: &mut cushy::context::EventContext<'_>) {
            self.field.unmount_in(context);
        }
    }
    // ANCHOR_END: widget-c

    FormField::new(
        "Label",
        Dynamic::<String>::default()
            .into_input()
            .placeholder("Field"),
    )
}

fn main() {
    cushy::example!(composition_widget).untested_still_frame();
}

#[test]
fn runs() {
    main();
}
