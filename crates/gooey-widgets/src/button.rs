use gooey_core::events::MouseEvent;
use gooey_core::style::Color;
use gooey_core::{AnyCallback, Callback, Value, Widget};

use crate::State;

#[derive(Debug, Default, Clone, Widget)]
#[widget(authority = gooey)]
#[must_use]
pub struct Button {
    pub label: Value<String>,
    pub on_click: Option<Callback<MouseEvent>>,
}

impl Button {
    pub fn new(label: impl Into<Value<String>>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
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

#[derive(Default, Debug)]
pub struct ButtonTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use futures_util::StreamExt;
    use gooey_core::style::DynamicStyle;
    use gooey_core::{Value, WidgetTransmogrifier};
    use gooey_web::WebApp;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlButtonElement;

    use crate::button::{Button, ButtonTransmogrifier};
    use crate::web_utils::mouse_event_from_web;

    impl WidgetTransmogrifier<WebApp> for ButtonTransmogrifier {
        type Widget = Button;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            _style: DynamicStyle,
            _context: &<WebApp as gooey_core::Frontend>::Context,
        ) -> <WebApp as gooey_core::Frontend>::Instance {
            // TODO apply style
            let label = widget.label.clone();
            let on_click = widget.on_click.clone();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let button = document
                .create_element("button")
                .expect("failed to create button")
                .dyn_into::<HtmlButtonElement>()
                .expect("incorrect element type");

            label.map_ref(|label| button.set_inner_text(label));

            if let Value::Dynamic(label) = label {
                let button = button.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut label = label.into_stream();
                    while let Some(new_label) = label.next().await {
                        button.set_inner_text(&new_label);
                    }
                });
            }

            if let Some(mut on_click) = on_click {
                let closure = Closure::new(move |event: web_sys::MouseEvent| {
                    on_click.invoke(mouse_event_from_web(event));
                });
                button
                    .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                    .expect("error installing button callback");
                closure.forget();
            }

            button.into()
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

    use crate::button::{button_background_color, ButtonTransmogrifier};
    use crate::{control_text_color, Button, State};

    struct ButtonRasterizer {
        state: State,
        button: Button,
        tracking_click: usize,
    }

    impl<Surface> WidgetTransmogrifier<RasterizedApp<Surface>> for ButtonTransmogrifier
    where
        Surface: gooey_raster::Surface,
    {
        type Widget = Button;

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
            Rasterizable::new(ButtonRasterizer {
                button: widget.clone(),
                state: State::Normal,
                tracking_click: 0,
            })
        }
    }

    impl WidgetRasterizer for ButtonRasterizer {
        type Widget = Button;

        fn measure(
            &mut self,
            _available_space: Size<ConstraintLimit>,
            renderer: &mut dyn Renderer,
            _context: &mut dyn AnyRasterContext,
        ) -> Size<UPx> {
            self.button.label.map_ref(|label| {
                let metrics: TextMetrics<Px> = renderer.measure_text(label, None);
                metrics.size.into_unsigned() + Size::new(10, 10) // TODO hard-coded padding
            })
        }

        fn draw(&mut self, renderer: &mut dyn Renderer, _context: &mut dyn AnyRasterContext) {
            renderer.fill.color = button_background_color(self.state);
            renderer.fill_rect(renderer.size().into_signed().into());
            self.button.label.map_ref(|label| {
                // TODO use the width
                let metrics: TextMetrics<Px> = renderer.measure_text(label, None);

                renderer.fill.color = control_text_color(self.state);
                let render_size = renderer.size().into_signed();
                renderer.draw_text(
                    label,
                    (Point::new(
                        render_size.width - metrics.size.width,
                        render_size.height + metrics.ascent,
                    )) / 2,
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
            if let (State::Active, Some(click)) = (self.state, &mut self.button.on_click) {
                click.invoke(event);
                self.state = State::Normal;
                context.invalidate();
            }
        }
    }
}
fn button_background_color(state: State) -> Color {
    match state {
        State::Normal => Color::rgba(100, 100, 100, 255),
        State::Hover => Color::rgba(120, 120, 120, 255),
        State::Active => Color::rgba(60, 60, 60, 255),
    }
}
