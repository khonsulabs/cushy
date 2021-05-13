use gooey_core::{euclid::Size2D, Widget};

#[derive(Eq, PartialEq, Clone)]
pub struct Button {
    pub label: String,
    pub disabled: bool,
}

pub enum ButtonEvent {
    Clicked,
}

impl Widget for Button {
    type MaterializerEvent = ButtonEvent;
    type State = Self;
    type Layout = ();

    fn state(&self) -> Self::State {
        self.clone()
    }

    fn layout(&self) -> Self::Layout {}

    fn content_size(
        &self,
        constraints: Size2D<Option<f32>, gooey_core::Points>,
    ) -> Size2D<f32, gooey_core::Points> {
        // TODO measure the text
        Size2D::new(
            constraints.width.unwrap_or_default(),
            constraints.height.unwrap_or_default(),
        )
    }
}

pub struct ButtonMaterializer;
