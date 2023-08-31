use gooey_core::events::MouseEvent;
use gooey_core::style::FontSize;
use gooey_core::{AnyCallback, Callback, Context, NewWidget, Value, Widget};

#[derive(Debug, Default, Clone, Widget)]
#[widget(authority = gooey)]
#[must_use]
pub struct Label {
    pub label: Value<String>,
    pub on_click: Option<Callback<MouseEvent>>,
}

impl Label {
    pub fn new(label: impl Into<Value<String>>, context: &Context) -> NewWidget<Self> {
        NewWidget::new(
            Self {
                label: label.into(),
                ..Self::default()
            },
            context,
        )
    }

    pub fn label(mut self, label: impl Into<Value<String>>) -> Self {
        self.label = label.into();
        self
    }

    pub fn on_click<CB: AnyCallback<MouseEvent>>(mut self, cb: CB) -> Self {
        self.on_click = Some(Callback::new(cb));
        self
    }
}

pub trait LabelExt {
    fn font_size(self, size: impl Into<Value<FontSize>>) -> NewWidget<Label>;
}

impl LabelExt for NewWidget<Label> {
    fn font_size(self, size: impl Into<Value<FontSize>>) -> NewWidget<Label> {
        self.style.push(size.into());
        self
    }
}

#[derive(Default)]
pub struct LabelTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use std::fmt::Write;

    use futures_util::StreamExt;
    use gooey_core::math::units::Px;
    use gooey_core::style::{Dimension, DynamicStyle, FontSize, Length};
    use gooey_core::{Value, WidgetTransmogrifier};
    use gooey_web::WebApp;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::{HtmlElement, Node};

    use crate::label::{Label, LabelTransmogrifier};
    use crate::web_utils::mouse_event_from_web;

    impl WidgetTransmogrifier<WebApp> for LabelTransmogrifier {
        type Widget = Label;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            style: DynamicStyle,
            _context: &gooey_web::WebContext,
        ) -> Node {
            let label = widget.label.clone();
            let on_click = widget.on_click.clone();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let element = document
                .create_element("div")
                .expect("failed to create button")
                .dyn_into::<HtmlElement>()
                .expect("incorrect element type");

            label.map_ref(|label| element.set_inner_text(label));

            // Apply initial styling.
            // style.map_ref(|style| {
            //     let mut css = String::new();
            //     if let Some(font_size) = style.get::<FontSize>() {
            //         let FontSize(Dimension::Pixels(pixels)) = font_size else { todo!("implement better dimension conversion") };
            //         write!(&mut css, "font-size:{pixels}px;").expect("error writing css");
            //     }

            //     // if !css.is_empty() {
            //         element
            //             .set_attribute("style", &css)
            //             .expect("error setting style");
            //     // }
            // });
            wasm_bindgen_futures::spawn_local({
                let element = element.clone();
                async move {
                    let mut style = style.into_stream();
                    while style.wait_next().await {
                        style.map_ref(|style| {
                            let mut css = String::new();
                            if let Some(font_size) = style.get::<FontSize>() {
                                let FontSize(Dimension::Length(Length::Pixels(Px(pixels)))) =
                                    font_size
                                else {
                                    todo!("implement better dimension conversion")
                                };
                                write!(&mut css, "font-size:{pixels}px;")
                                    .expect("error writing css");
                            }

                            // if !css.is_empty() {
                            element
                                .set_attribute("style", &css)
                                .expect("error setting style");
                            // }
                        });
                    }
                }
            });

            if let Value::Dynamic(label) = label {
                let element = element.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut label = label.into_stream();
                    while let Some(new_label) = label.next().await {
                        element.set_inner_text(&new_label);
                    }
                });
            }

            if let Some(mut on_click) = on_click {
                let closure = Closure::new(move |event: web_sys::MouseEvent| {
                    on_click.invoke(mouse_event_from_web(event));
                });
                element
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .expect("error installing button callback");
                closure.forget();
            }

            element.into()
        }
    }
}

#[cfg(feature = "raster")]
mod raster {
    use gooey_core::events::MouseEvent;
    use gooey_core::graphics::TextMetrics;
    use gooey_core::math::units::{Px, UPx};
    use gooey_core::math::{IntoSigned, IntoUnsigned, Point, Size};
    use gooey_core::style::DynamicStyle;
    use gooey_core::{Value, WidgetTransmogrifier};
    use gooey_raster::{
        AnyRasterContext, ConstraintLimit, RasterContext, Rasterizable, RasterizedApp, Renderer,
        WidgetRasterizer,
    };

    use crate::label::LabelTransmogrifier;
    use crate::{control_text_color, Label, State};

    struct LabelRasterizer {
        state: State, // The element state should probably be stored somewhere else.
        label: Label,
        tracking_click: usize,
    }

    impl<Surface> WidgetTransmogrifier<RasterizedApp<Surface>> for LabelTransmogrifier
    where
        Surface: gooey_raster::Surface,
    {
        type Widget = Label;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            _style: DynamicStyle,
            context: &RasterContext<Surface>,
        ) -> Rasterizable {
            // TODO apply style
            if let Value::Dynamic(value) = &widget.label {
                value.for_each({
                    let handle = context.handle().clone();
                    move |_| {
                        handle.invalidate();
                    }
                });
            }

            Rasterizable::new(LabelRasterizer {
                label: widget.clone(),
                state: State::Normal,
                tracking_click: 0,
            })
        }
    }

    impl WidgetRasterizer for LabelRasterizer {
        type Widget = Label;

        fn measure(
            &mut self,
            _available_space: Size<ConstraintLimit>,
            renderer: &mut dyn Renderer,
            _context: &mut dyn AnyRasterContext,
        ) -> Size<UPx> {
            self.label.label.map_ref(|label| {
                let metrics: TextMetrics<Px> = renderer.measure_text(label, None);
                metrics.size.into_unsigned() + Size::new(10, 10) // TODO hard-coded padding
            })
        }

        fn draw(&mut self, renderer: &mut dyn Renderer, _context: &mut dyn AnyRasterContext) {
            self.label.label.map_ref(|label| {
                // TODO use the width
                let metrics: TextMetrics<Px> = renderer.measure_text(label, None);

                renderer.fill.color = control_text_color(self.state);
                renderer.draw_text(
                    label,
                    Point::from(renderer.size().into_signed() - metrics.size) / 2
                        + Point::new(Px(0), metrics.ascent),
                    None,
                );
            });
        }

        fn mouse_down(&mut self, _event: MouseEvent, context: &mut dyn AnyRasterContext) {
            self.tracking_click += 1;
            self.state = State::Active;
            context.invalidate();
        }

        fn cursor_moved(&mut self, event: MouseEvent, context: &mut dyn AnyRasterContext) {
            let hover_state = if self.tracking_click > 0 {
                State::Active
            } else {
                State::Hover
            };
            let changed = event.position.is_some() != (self.state == hover_state);
            if changed {
                if event.position.is_some() {
                    self.state = hover_state;
                } else {
                    self.state = State::Normal;
                }
                context.invalidate();
            }
        }

        fn mouse_up(&mut self, event: MouseEvent, context: &mut dyn AnyRasterContext) {
            self.tracking_click -= 1;
            // TODO: only primary?
            if let (State::Active, Some(click)) = (self.state, &mut self.label.on_click) {
                click.invoke(event);
                self.state = State::Normal;
                context.invalidate();
            }
        }
    }
}
