use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use gooey_core::{
    figures::{Point, Rectlike},
    styles::{style_sheet, Intent, SystemTheme, TabIndex},
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
    ids: HashMap<u32, usize>,
    area: HashMap<u32, ContentArea>,
    hierarchy: HashMap<Option<u32>, Vec<u32>>,
    tab_orders: HashMap<u32, TabEntry>,
    system_theme: SystemTheme,

    default: Option<WidgetId>,
    cancel: Option<WidgetId>,

    hover: HashSet<WidgetId>,
    focus: Option<WidgetId>,
    focus_events: Vec<FocusEvent>,
    active: Option<WidgetId>,

    last_mouse_position: Option<Point<f32, Scaled>>,
    mouse_button_handlers: HashMap<MouseButton, WidgetId>,
    redraw_status: RedrawStatus,

    modifiers: ModifiersState,
}

#[derive(Debug)]
pub enum FocusEvent {
    Focused(WidgetId),
    Blurred(WidgetId),
}

impl FocusEvent {
    pub const fn widget(&self) -> &WidgetId {
        match self {
            FocusEvent::Focused(widget) | FocusEvent::Blurred(widget) => widget,
        }
    }
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

    pub fn widget_rendered(
        &self,
        widget: WidgetId,
        area: ContentArea,
        should_accept_focus: bool,
        parent_id: Option<&WidgetId>,
        tab_order: Option<TabIndex>,
        intent: Option<Intent>,
    ) {
        let mut data = self.data.lock();
        data.widget_rendered(
            widget,
            area,
            should_accept_focus,
            parent_id,
            tab_order,
            intent,
        );
    }

    pub fn widget_area(&self, widget: &WidgetId) -> Option<ContentArea> {
        let data = self.data.lock();
        data.area.get(&widget.id).cloned()
    }

    pub fn widgets_under_point(&self, location: Point<f32, Scaled>) -> Vec<WidgetId> {
        let data = self.data.lock();
        data.widgets_under_point(location).cloned().collect()
    }

    pub fn next_tab_entry(&self) -> Option<WidgetId> {
        let data = self.data.lock();
        data.next_tab_entry().cloned()
    }

    pub fn previous_tab_entry(&self) -> Option<WidgetId> {
        let data = self.data.lock();
        data.previous_tab_entry().cloned()
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
            if let Some(focused) = focus.clone() {
                data.focus_events.push(FocusEvent::Focused(focused));
            }
            if let Some(blurring) = std::mem::replace(&mut data.focus, focus) {
                data.focus_events.push(FocusEvent::Blurred(blurring));
            }
        }
    }

    pub fn blur(&self) {
        let mut data = self.data.lock();
        if data.focus.is_some() {
            if let Some(blurring) = data.focus.take() {
                data.focus_events.push(FocusEvent::Blurred(blurring));
            }
            data.redraw_status = RedrawStatus::Dirty;
        }
    }

    pub fn blur_and_deactivate(&self) {
        let mut data = self.data.lock();
        if data.focus.is_some() || data.active.is_some() {
            if let Some(blurring) = data.focus.take() {
                data.focus_events.push(FocusEvent::Blurred(blurring));
            }
            data.active = None;
            data.redraw_status = RedrawStatus::Dirty;
        }
    }

    pub fn default_widget(&self) -> Option<WidgetId> {
        let data = self.data.lock();
        data.default.clone()
    }

    // We are tracking this state, but it doesn't seem prudent to actually hook it to a keyboard shortcut.
    pub fn cancel_widget(&self) -> Option<WidgetId> {
        let data = self.data.lock();
        data.cancel.clone()
    }

    pub fn focus_events(&self) -> Vec<FocusEvent> {
        let mut data = self.data.lock();
        std::mem::take(&mut data.focus_events)
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
        self.default = None;
        self.cancel = None;
        self.order.clear();
        self.tab_orders.clear();
        self.ids.clear();
        self.hierarchy.clear();
        self.area.clear();
        self.redraw_status = RedrawStatus::Clean;
    }

    pub fn widget_rendered(
        &mut self,
        widget: WidgetId,
        area: ContentArea,
        should_accept_focus: bool,
        parent_id: Option<&WidgetId>,
        tab_order: Option<TabIndex>,
        intent: Option<Intent>,
    ) {
        if should_accept_focus {
            let tab_entry = TabEntry {
                order_index: self.order.len(),
                render_origin: area.location.cast(),
                tab_order,
            };
            self.tab_orders.insert(widget.id, tab_entry);
        }

        let parent_children = self.hierarchy.entry(parent_id.map(|id| id.id)).or_default();
        parent_children.push(widget.id);

        self.area.insert(widget.id, area);
        self.ids.insert(widget.id, self.order.len());
        if let Some(intent) = intent {
            match intent {
                Intent::Default => {
                    self.default = Some(widget.clone());
                }
                Intent::Cancel => {
                    self.cancel = Some(widget.clone());
                }
            }
        }
        self.order.push(widget);
    }

    pub fn next_tab_entry(&self) -> Option<&WidgetId> {
        // Start at the root and iterate over children until we find the current focus.
        let mut stack = self.hierarchy.get(&None).cloned().unwrap_or_default();
        let mut first = None;
        let mut return_next = self.focus.is_none();
        while let Some(id) = stack.pop() {
            if let Some(tab_order) = self.tab_orders.get(&id) {
                let widget_id = &self.order[tab_order.order_index];
                if first.is_none() {
                    first = Some(widget_id);
                }

                if return_next {
                    return Some(widget_id);
                } else if self.focus.as_ref() == Some(widget_id) {
                    return_next = true;
                }
            }

            if let Some(children) = self.hierarchy.get(&Some(id)) {
                let mut tab_entries = children
                    .iter()
                    .map(|id| (*id, self.tab_orders.get(id)))
                    .collect::<Vec<_>>();
                tab_entries
                    .sort_by_key(|(_, entry)| entry.map_or((1, None), |entry| (0, Some(entry))));
                stack.extend(tab_entries.into_iter().map(|(id, _)| id).rev());
            }
        }

        first
    }

    pub fn previous_tab_entry(&self) -> Option<&WidgetId> {
        // Start at the root and iterate over children until we find the current focus.
        let mut stack = self.hierarchy.get(&None).cloned().unwrap_or_default();
        let mut first = None;
        let mut return_next = self.focus.is_none();
        while let Some(id) = stack.pop() {
            if let Some(tab_order) = self.tab_orders.get(&id) {
                let widget_id = &self.order[tab_order.order_index];
                if first.is_none() {
                    first = Some(widget_id);
                }

                if return_next {
                    return Some(widget_id);
                } else if self.focus.as_ref() == Some(widget_id) {
                    return_next = true;
                }
            }

            if let Some(children) = self.hierarchy.get(&Some(id)) {
                let mut tab_entries = children
                    .iter()
                    .map(|id| (*id, self.tab_orders.get(id)))
                    .collect::<Vec<_>>();
                tab_entries
                    .sort_by_key(|(_, entry)| entry.map_or((1, None), |entry| (0, Some(entry))));
                stack.extend(tab_entries.into_iter().map(|(id, _)| id));
            }
        }

        first
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TabEntry {
    order_index: usize,
    render_origin: Point<i32, Scaled>,
    tab_order: Option<TabIndex>,
}

impl PartialOrd for TabEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TabEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // The order of comparison priority is:
        // - TabOrder
        // - y, if y is outside +/-10 Scaled
        // - x
        match (self.tab_order, other.tab_order) {
            (Some(this_order), Some(other_order)) => this_order.cmp(&other_order),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => match self.render_origin.y - other.render_origin.y {
                i32::MIN..=-10 => Ordering::Less,
                -9..=9 => self.render_origin.x.cmp(&other.render_origin.x),
                10..=i32::MAX => Ordering::Greater,
            },
        }
    }
}
