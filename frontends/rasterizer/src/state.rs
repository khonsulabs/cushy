use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use gooey_core::{
    euclid::{Point2D, Rect},
    styles::{style_sheet, SystemTheme},
    Points, WidgetId,
};
use winit::event::MouseButton;

#[derive(Clone, Default, Debug)]
pub struct State {
    data: Arc<Mutex<StateData>>,
}

#[derive(Default, Debug)]
struct StateData {
    order: Vec<WidgetId>,
    bounds: HashMap<u32, Rect<f32, Points>>,
    system_theme: SystemTheme,

    hover: HashSet<WidgetId>,
    focus: Option<WidgetId>,
    active: Option<WidgetId>,

    last_mouse_position: Option<Point2D<f32, Points>>,
    mouse_button_handlers: HashMap<MouseButton, WidgetId>,
    needs_redraw: bool,
}

impl State {
    pub fn new_frame(&self) {
        let mut data = self.data.lock().unwrap();
        data.new_frame()
    }

    pub fn widget_rendered(&self, widget: WidgetId, bounds: Rect<f32, Points>) {
        let mut data = self.data.lock().unwrap();
        data.widget_rendered(widget, bounds)
    }

    pub fn widget_bounds(&self, widget: &WidgetId) -> Option<Rect<f32, Points>> {
        let data = self.data.lock().unwrap();
        data.bounds.get(&widget.id).copied()
    }

    pub fn widgets_under_point(&self, location: Point2D<f32, Points>) -> Vec<WidgetId> {
        let data = self.data.lock().unwrap();
        data.widgets_under_point(location).cloned().collect()
    }

    pub fn set_needs_redraw(&self) {
        let mut data = self.data.lock().unwrap();
        data.needs_redraw = true;
    }

    pub fn needs_redraw(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.needs_redraw
    }

    pub fn set_last_mouse_position(&self, location: Option<Point2D<f32, Points>>) {
        let mut data = self.data.lock().unwrap();
        data.last_mouse_position = location;
    }

    pub fn last_mouse_position(&self) -> Option<Point2D<f32, Points>> {
        let data = self.data.lock().unwrap();
        data.last_mouse_position
    }

    pub fn clear_hover(&self) -> HashSet<WidgetId> {
        let mut data = self.data.lock().unwrap();
        std::mem::take(&mut data.hover)
    }

    pub fn hover(&self) -> HashSet<WidgetId> {
        let data = self.data.lock().unwrap();
        data.hover.clone()
    }

    pub fn set_hover(&self, hover: HashSet<WidgetId>) {
        let mut data = self.data.lock().unwrap();
        if data.hover != hover {
            data.needs_redraw = true;
            data.hover = hover;
        }
    }

    pub fn active(&self) -> Option<WidgetId> {
        let data = self.data.lock().unwrap();
        data.active.clone()
    }

    pub fn set_active(&self, active: Option<WidgetId>) {
        let mut data = self.data.lock().unwrap();
        if data.active != active {
            data.needs_redraw = true;
            data.active = active;
        }
    }

    pub fn focus(&self) -> Option<WidgetId> {
        let data = self.data.lock().unwrap();
        data.focus.clone()
    }

    pub fn set_focus(&self, focus: Option<WidgetId>) {
        let mut data = self.data.lock().unwrap();
        if data.focus != focus {
            data.needs_redraw = true;
            data.focus = focus;
        }
    }

    pub fn blur(&self) {
        let mut data = self.data.lock().unwrap();
        if data.focus.is_some() || data.active.is_some() {
            data.focus = None;
            data.active = None;
            data.needs_redraw = true;
        }
    }

    pub fn system_theme(&self) -> SystemTheme {
        let data = self.data.lock().unwrap();
        data.system_theme
    }

    pub fn set_system_theme(&self, system_theme: SystemTheme) {
        let mut data = self.data.lock().unwrap();
        if data.system_theme != system_theme {
            data.needs_redraw = true;
            data.system_theme = system_theme;
        }
    }

    pub fn register_mouse_handler(&self, button: MouseButton, widget: WidgetId) {
        let mut data = self.data.lock().unwrap();
        data.mouse_button_handlers.insert(button, widget);
    }

    pub fn take_mouse_button_handler(&self, button: MouseButton) -> Option<WidgetId> {
        let mut data = self.data.lock().unwrap();
        data.mouse_button_handlers.remove(&button)
    }

    pub fn mouse_button_handlers(&self) -> HashMap<MouseButton, WidgetId> {
        let data = self.data.lock().unwrap();
        data.mouse_button_handlers.clone()
    }

    pub fn ui_state_for(&self, widget_id: &WidgetId) -> style_sheet::State {
        let data = self.data.lock().unwrap();
        style_sheet::State {
            hovered: data.hover.contains(widget_id),
            active: data.active.as_ref() == Some(widget_id),
            focused: data.focus.as_ref() == Some(widget_id),
        }
    }
}

impl StateData {
    pub fn new_frame(&mut self) {
        self.order.clear();
        self.bounds.clear();
        self.needs_redraw = false;
    }

    pub fn widget_rendered(&mut self, widget: WidgetId, bounds: Rect<f32, Points>) {
        self.bounds.insert(widget.id, bounds);
        self.order.push(widget);
    }

    pub fn widgets_under_point(
        &self,
        location: Point2D<f32, Points>,
    ) -> impl Iterator<Item = &WidgetId> {
        self.order
            .iter()
            .rev()
            .filter(move |id| self.bounds.get(&id.id).unwrap().contains(location))
    }
}
