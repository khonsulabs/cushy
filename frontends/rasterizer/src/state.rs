use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use gooey_core::{
    figures::{Point, Rectlike},
    styles::{style_sheet, SystemTheme},
    Scaled, WidgetId,
};
use parking_lot::Mutex;
use winit::event::{ModifiersState, MouseButton};

use crate::{Configuration, ContentArea};

#[derive(Clone, Default, Debug)]
pub struct State {
    data: Arc<Mutex<Data>>,
    configuration: Arc<Configuration>,
}

#[derive(Default, Debug)]
struct Data {
    order: Vec<WidgetId>,
    area: HashMap<u32, ContentArea>,
    system_theme: SystemTheme,

    hover: HashSet<WidgetId>,
    focus: Option<WidgetId>,
    active: Option<WidgetId>,

    last_mouse_position: Option<Point<f32, Scaled>>,
    mouse_button_handlers: HashMap<MouseButton, WidgetId>,
    redraw_status: RedrawStatus,

    modifiers: ModifiersState,
}

#[derive(Debug)]
enum RedrawStatus {
    Clean,
    Dirty,
    Scheduled(Instant),
}

impl Default for RedrawStatus {
    fn default() -> Self {
        Self::Clean
    }
}

impl RedrawStatus {
    fn needs_redraw(&self) -> bool {
        matches!(self, Self::Dirty)
    }

    fn duration_until_next_redraw(&self) -> Option<Duration> {
        match self {
            RedrawStatus::Clean => None,
            RedrawStatus::Dirty => Some(Duration::new(0, 0)),
            RedrawStatus::Scheduled(instant) => Some(
                instant
                    .checked_duration_since(Instant::now())
                    .unwrap_or_default(),
            ),
        }
    }
}

impl State {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            data: Arc::default(),
            configuration: Arc::new(configuration),
        }
    }

    pub fn configuration(&self) -> &Configuration {
        &self.configuration
    }

    pub fn new_frame(&self) {
        let mut data = self.data.lock();
        data.new_frame();
    }

    pub fn widget_rendered(&self, widget: WidgetId, area: ContentArea) {
        let mut data = self.data.lock();
        data.widget_rendered(widget, area);
    }

    pub fn widget_area(&self, widget: &WidgetId) -> Option<ContentArea> {
        let data = self.data.lock();
        data.area.get(&widget.id).cloned()
    }

    pub fn widgets_under_point(&self, location: Point<f32, Scaled>) -> Vec<WidgetId> {
        let data = self.data.lock();
        data.widgets_under_point(location).cloned().collect()
    }

    pub fn set_needs_redraw(&self) {
        let mut data = self.data.lock();
        data.redraw_status = RedrawStatus::Dirty;
    }

    pub fn needs_redraw(&self) -> bool {
        let data = self.data.lock();
        data.redraw_status.needs_redraw()
    }

    pub fn duration_until_next_redraw(&self) -> Option<Duration> {
        let data = self.data.lock();
        data.redraw_status.duration_until_next_redraw()
    }

    pub fn schedule_redraw_in(&self, duration: Duration) {
        self.schedule_redraw_at(Instant::now() + duration);
    }

    pub fn schedule_redraw_at(&self, at: Instant) {
        let mut data = self.data.lock();
        match data.redraw_status {
            RedrawStatus::Clean => {
                data.redraw_status = RedrawStatus::Scheduled(at);
            }
            RedrawStatus::Dirty => {}
            RedrawStatus::Scheduled(previous_at) => {
                data.redraw_status = RedrawStatus::Scheduled(at.min(previous_at));
            }
        }
    }

    pub fn set_last_mouse_position(&self, location: Option<Point<f32, Scaled>>) {
        let mut data = self.data.lock();
        data.last_mouse_position = location;
    }

    pub fn last_mouse_position(&self) -> Option<Point<f32, Scaled>> {
        let data = self.data.lock();
        data.last_mouse_position
    }

    pub fn clear_hover(&self) -> HashSet<WidgetId> {
        let mut data = self.data.lock();
        std::mem::take(&mut data.hover)
    }

    pub fn hover(&self) -> HashSet<WidgetId> {
        let data = self.data.lock();
        data.hover.clone()
    }

    pub fn set_hover(&self, hover: HashSet<WidgetId>) {
        let mut data = self.data.lock();
        if data.hover != hover {
            data.redraw_status = RedrawStatus::Dirty;
            data.hover = hover;
        }
    }

    pub fn active(&self) -> Option<WidgetId> {
        let data = self.data.lock();
        data.active.clone()
    }

    pub fn set_active(&self, active: Option<WidgetId>) {
        let mut data = self.data.lock();
        if data.active != active {
            data.redraw_status = RedrawStatus::Dirty;
            data.active = active;
        }
    }

    pub fn focus(&self) -> Option<WidgetId> {
        let data = self.data.lock();
        data.focus.clone()
    }

    pub fn set_focus(&self, focus: Option<WidgetId>) {
        let mut data = self.data.lock();
        if data.focus != focus {
            data.redraw_status = RedrawStatus::Dirty;
            data.focus = focus;
        }
    }

    pub fn blur(&self) {
        let mut data = self.data.lock();
        if data.focus.is_some() || data.active.is_some() {
            data.focus = None;
            data.active = None;
            data.redraw_status = RedrawStatus::Dirty;
        }
    }

    pub fn system_theme(&self) -> SystemTheme {
        let data = self.data.lock();
        data.system_theme
    }

    pub fn set_system_theme(&self, system_theme: SystemTheme) {
        let mut data = self.data.lock();
        if data.system_theme != system_theme {
            data.redraw_status = RedrawStatus::Dirty;
            data.system_theme = system_theme;
        }
    }

    pub fn register_mouse_handler(&self, button: MouseButton, widget: WidgetId) {
        let mut data = self.data.lock();
        data.mouse_button_handlers.insert(button, widget);
    }

    pub fn take_mouse_button_handler(&self, button: MouseButton) -> Option<WidgetId> {
        let mut data = self.data.lock();
        data.mouse_button_handlers.remove(&button)
    }

    pub fn mouse_button_handlers(&self) -> HashMap<MouseButton, WidgetId> {
        let data = self.data.lock();
        data.mouse_button_handlers.clone()
    }

    pub fn ui_state_for(&self, widget_id: &WidgetId) -> style_sheet::State {
        let data = self.data.lock();
        style_sheet::State {
            hovered: data.hover.contains(widget_id),
            active: data.active.as_ref() == Some(widget_id),
            focused: data.focus.as_ref() == Some(widget_id),
        }
    }

    pub fn assets_path(&self) -> PathBuf {
        let path = self
            .configuration
            .assets_path
            .clone()
            .unwrap_or_else(|| PathBuf::from("assets"));
        if path.is_absolute() {
            path
        } else {
            let base_path = if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                PathBuf::from(manifest_dir)
            } else {
                std::env::current_exe()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .to_owned()
            };
            base_path.join(path)
        }
    }

    pub fn set_keyboard_modifiers(&self, modifiers: ModifiersState) {
        let mut data = self.data.lock();
        data.modifiers = modifiers;
    }

    pub fn keyboard_modifiers(&self) -> ModifiersState {
        let data = self.data.lock();
        data.modifiers
    }
}

impl Data {
    pub fn new_frame(&mut self) {
        self.order.clear();
        self.area.clear();
        self.redraw_status = RedrawStatus::Clean;
    }

    pub fn widget_rendered(&mut self, widget: WidgetId, area: ContentArea) {
        self.area.insert(widget.id, area);
        self.order.push(widget);
    }

    pub fn widgets_under_point(
        &self,
        location: Point<f32, Scaled>,
    ) -> impl Iterator<Item = &WidgetId> {
        self.order
            .iter()
            .rev()
            .filter(move |id| self.area.get(&id.id).unwrap().bounds().contains(location))
    }
}
