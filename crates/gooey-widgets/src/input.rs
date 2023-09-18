use gooey_core::style::Color;
use gooey_core::{AnyCallback, Callback, Value, Widget};

use crate::State;

#[derive(Debug, Default, Clone, Widget)]
#[widget(authority = gooey)]
#[must_use]
pub struct Input {
    pub value: Value<String>,
    pub on_update: Option<Callback<String>>,
}

impl Input {
    pub fn new(value: impl Into<Value<String>>) -> Self {
        Self {
            value: value.into(),
            ..Self::default()
        }
    }

    pub fn value(mut self, value: impl Into<Value<String>>) -> Self {
        self.value = value.into();
        self
    }

    pub fn on_update<CB: AnyCallback<String>>(mut self, cb: CB) -> Self {
        self.on_update = Some(Callback::new(cb));
        self
    }
}

#[derive(Default, Debug)]
pub struct InputTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use futures_util::StreamExt;
    use gooey_core::style::DynamicStyle;
    use gooey_core::{Value, WidgetTransmogrifier};
    use gooey_web::WebApp;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlInputElement;

    use crate::input::{Input, InputTransmogrifier};

    impl WidgetTransmogrifier<WebApp> for InputTransmogrifier {
        type Widget = Input;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            _style: DynamicStyle,
            _context: &<WebApp as gooey_core::Frontend>::Context,
        ) -> <WebApp as gooey_core::Frontend>::Instance {
            // TODO apply style
            let label = widget.value.clone();
            let on_update = widget.on_update.clone();
            let document = web_sys::window()
                .expect("no window")
                .document()
                .expect("no document");
            let input = document
                .create_element("input")
                .expect("failed to create input")
                .dyn_into::<HtmlInputElement>()
                .expect("incorrect element type");

            label.map_ref(|label| input.set_inner_text(label));

            if let Value::Dynamic(label) = label {
                let input = input.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut label = label.into_stream();
                    while let Some(new_label) = label.next().await {
                        input.set_inner_text(&new_label);
                    }
                });
            }

            if let Some(mut on_update) = on_update {
                let closure = Closure::new(move || {
                    on_update.invoke(String::new());
                });
                input
                    .add_event_listener_with_callback("input", closure.as_ref().unchecked_ref())
                    .expect("error installing input callback");
                closure.forget();
            }

            input.into()
        }
    }
}

#[cfg(feature = "raster")]
mod raster {
    use gooey_core::events::{Key, KeyEvent, MouseEvent};
    use gooey_core::graphics::TextMetrics;
    use gooey_core::math::units::{Px, UPx};
    use gooey_core::math::{IntoSigned, IntoUnsigned, Point, Size};
    use gooey_core::style::DynamicStyle;
    use gooey_core::{Value, WidgetTransmogrifier};
    use gooey_raster::{
        AnyRasterContext, ConstraintLimit, RasterContext, Rasterizable, RasterizedApp, Renderer,
        WidgetRasterizer,
    };

    use crate::input::{input_background_color, InputTransmogrifier};
    use crate::{control_text_color, Input, State};

    struct InputRasterizer {
        state: State,
        input: Input,
        value: String,
    }

    impl<Surface> WidgetTransmogrifier<RasterizedApp<Surface>> for InputTransmogrifier
    where
        Surface: gooey_raster::Surface,
    {
        type Widget = Input;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            _style: DynamicStyle,
            context: &RasterContext<Surface>,
        ) -> Rasterizable {
            // TODO apply style
            if let Value::Dynamic(value) = &widget.value {
                value.for_each({
                    let handle = context.handle().clone();
                    move |_| {
                        handle.invalidate();
                    }
                });
            }
            Rasterizable::new(InputRasterizer {
                input: widget.clone(),
                state: State::Normal,
                value: widget.value.get(),
            })
        }
    }

    impl WidgetRasterizer for InputRasterizer {
        type Widget = Input;

        fn measure(
            &mut self,
            _available_space: Size<ConstraintLimit>,
            renderer: &mut dyn Renderer,
            _context: &mut dyn AnyRasterContext,
        ) -> Size<UPx> {
            self.input.value.map_ref(|label| {
                let metrics: TextMetrics<Px> = renderer.measure_text(label, None);
                metrics.size.into_unsigned() + Size::new(10, 10) // TODO hard-coded padding
            })
        }

        fn draw(&mut self, renderer: &mut dyn Renderer, _context: &mut dyn AnyRasterContext) {
            renderer.fill.color = input_background_color(self.state);
            renderer.fill_rect(renderer.size().into_signed().into());
            self.input.value.map_ref(|label| {
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
            self.state = State::Active;
            // TODO position
            context.invalidate();
        }

        fn cursor_moved(&mut self, event: MouseEvent, context: &mut dyn AnyRasterContext) {
            let changed = event.position.is_some() != (self.state == State::Hover);
            if changed {
                if event.position.is_some() {
                    self.state = State::Hover;
                } else {
                    self.state = State::Normal;
                }
                context.invalidate();
            }
        }

        fn key_down(&mut self, event: KeyEvent, context: &mut dyn AnyRasterContext) {
            match event.logical_key {
                Key::Backspace => self.value.truncate(self.value.len() - 1),
                _ => {
                    if let Some(text) = event.text {
                        self.value.push_str(text.as_str());
                    } else {
                        eprintln!("ignored {event:?}");
                        return;
                    }
                }
            }
            if let Value::Dynamic(value) = self.input.value {
                value.set(self.value.clone());
            }
            if let Some(on_update) = self.input.on_update.as_mut() {
                on_update.invoke(self.value.clone());
            }
            context.invalidate();
        }
    }
}

fn input_background_color(state: State) -> Color {
    match state {
        State::Normal => Color::rgba(100, 100, 100, 255),
        State::Hover => Color::rgba(120, 120, 120, 255),
        State::Active => Color::rgba(60, 60, 60, 255),
    }
}
