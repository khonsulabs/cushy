use gooey_core::style::Color;
use gooey_core::{AnyCallback, Callback, Widget, WidgetValue};

#[derive(Debug, Default, Clone, Widget)]
#[widget(authority = gooey)]
pub struct Button {
    pub label: WidgetValue<String>,
    pub on_click: Option<Callback<()>>,
}

impl Button {
    pub fn new(label: impl Into<WidgetValue<String>>) -> Self {
        Self {
            label: label.into(),
            ..Self::default()
        }
    }

    pub fn label(mut self, label: impl Into<WidgetValue<String>>) -> Self {
        self.label = label.into();
        self
    }

    pub fn on_click<CB: AnyCallback<()>>(mut self, cb: CB) -> Self {
        self.on_click = Some(Callback::new(cb));
        self
    }
}

#[derive(Default, Debug)]
pub struct ButtonTransmogrifier;

#[cfg(feature = "web")]
mod web {
    use futures_util::StreamExt;
    use gooey_core::reactor::Value;
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_web::WebApp;
    use stylecs::Style;
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    use web_sys::HtmlButtonElement;

    use crate::button::{Button, ButtonTransmogrifier};

    impl WidgetTransmogrifier<WebApp> for ButtonTransmogrifier {
        type Widget = Button;

        fn transmogrify(
            &self,
            widget: &Self::Widget,
            style: Value<Style>,
            _context: &<WebApp as gooey_core::Frontend>::Context,
        ) -> <WebApp as gooey_core::Frontend>::Instance {
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

            if let WidgetValue::Value(label) = label {
                let button = button.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let mut label = label.into_stream();
                    while let Some(new_label) = label.next().await {
                        button.set_inner_text(&new_label);
                    }
                });
            }

            if let Some(mut on_click) = on_click {
                let closure = Closure::new(move || {
                    on_click.invoke(());
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
    use gooey_core::graphics::Point;
    use gooey_core::math::IntoSigned;
    use gooey_core::style::Px;
    use gooey_core::{WidgetTransmogrifier, WidgetValue};
    use gooey_raster::{RasterContext, RasterizedApp, SurfaceHandle, WidgetRasterizer};

    use crate::button::{button_background_color, button_text_color, ButtonTransmogrifier, State};
    use crate::Button;

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
            style: gooey_core::reactor::Value<stylecs::Style>,
            context: &RasterContext<Surface>,
        ) -> Surface::Rasterizable {
            if let WidgetValue::Value(value) = &widget.label {
                value.for_each({
                    let handle = context.handle().clone();
                    move |_| {
                        handle.invalidate();
                    }
                })
            }
            Surface::new_rasterizable(ButtonRasterizer {
                button: widget.clone(),
                state: State::Normal,
                tracking_click: 0,
            })
        }
    }

    impl WidgetRasterizer for ButtonRasterizer {
        type Widget = Button;

        fn draw<Renderer>(&mut self, renderer: &mut Renderer)
        where
            Renderer: gooey_core::graphics::Renderer,
        {
            renderer.fill.color = button_background_color(self.state);
            renderer.fill_rect(renderer.size().into());
            self.button.label.map_ref(|label| {
                // TODO use the width
                let metrics = renderer.measure_text::<Px>(label, None);

                renderer.fill.color = button_text_color(self.state);
                renderer.draw_text(
                    label,
                    Point::from(renderer.size().into_signed() - metrics.size) / 2
                        + Point::new(Px(0), metrics.ascent),
                    None,
                );
            });
        }

        fn mouse_down(&mut self, _location: Point<Px>, surface: &dyn SurfaceHandle) {
            self.tracking_click += 1;
            self.state = State::Active;
            surface.invalidate();
        }

        fn cursor_moved(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {
            let hover_state = if self.tracking_click > 0 {
                State::Active
            } else {
                State::Hover
            };
            let changed = location.is_some() != (self.state == hover_state);
            if changed {
                if location.is_some() {
                    self.state = hover_state;
                } else {
                    self.state = State::Normal;
                }
                surface.invalidate();
            }
        }

        fn mouse_up(&mut self, location: Option<Point<Px>>, surface: &dyn SurfaceHandle) {
            self.tracking_click -= 1;
            if let (State::Active, Some(click)) = (self.state, &mut self.button.on_click) {
                click.invoke(());
                self.state = State::Normal;
                surface.invalidate();
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum State {
    Normal,
    Hover,
    Active,
}

fn button_text_color(state: State) -> Color {
    match state {
        State::Normal => Color::rgba(0, 0, 0, 255),
        State::Hover => Color::rgba(20, 20, 20, 255),
        State::Active => Color::rgba(0, 0, 0, 255),
    }
}
fn button_background_color(state: State) -> Color {
    match state {
        State::Normal => Color::rgba(100, 100, 100, 255),
        State::Hover => Color::rgba(120, 120, 120, 255),
        State::Active => Color::rgba(60, 60, 60, 255),
    }
}
