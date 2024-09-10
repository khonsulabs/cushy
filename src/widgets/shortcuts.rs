//! A keyboard shortcut handling widget.

use ahash::AHashMap;
use kludgine::app::winit::keyboard::{
    Key, KeyCode, ModifiersState, NamedKey, NativeKey, NativeKeyCode, PhysicalKey, SmolStr,
};

use crate::widget::{
    EventHandling, MakeWidget, SharedCallback, WidgetRef, WrapperWidget, HANDLED, IGNORED,
};
use crate::window::KeyEvent;
use crate::{ModifiersExt, ModifiersStateExt};

/// A collection of keyboard shortcut handlers.
#[derive(Default, Debug, Clone)]
pub struct ShortcutMap(AHashMap<Shortcut, ShortcutConfig>);

impl ShortcutMap {
    /// Inserts a handler that invokes `callback` once when `key` is pressed
    /// with `modifiers`.
    #[must_use]
    pub fn with_shortcut<F>(
        mut self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) -> Self
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.insert(key.into(), modifiers, callback);
        self
    }

    /// Inserts a handler that invokes `callback` once when `key` is pressed
    /// with `modifiers`.
    pub fn insert<F>(&mut self, key: impl Into<ShortcutKey>, modifiers: ModifiersState, callback: F)
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.insert_shortcut_inner(key.into(), modifiers, false, SharedCallback::new(callback));
    }

    /// Inserts a handler that invokes `callback` when `key` is pressed with
    /// `modifiers`. This callback will be invoked for repeated key events.
    #[must_use]
    pub fn with_repeating_shortcut<F>(
        mut self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) -> Self
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.insert_repeating(key.into(), modifiers, callback);
        self
    }

    /// Inserts a handler that invokes `callback` when `key` is pressed with
    /// `modifiers`. This callback will be invoked for repeated key events.
    pub fn insert_repeating<F>(
        &mut self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.insert_shortcut_inner(key.into(), modifiers, true, SharedCallback::new(callback));
    }

    fn insert_shortcut_inner(
        &mut self,
        key: ShortcutKey,
        modifiers: ModifiersState,
        repeat: bool,
        callback: SharedCallback<KeyEvent, EventHandling>,
    ) {
        let (first, second) = Shortcut { key, modifiers }.into_variations();
        let config = ShortcutConfig { repeat, callback };

        if let Some(second) = second {
            self.0.insert(second, config.clone());
        }

        self.0.insert(first, config);
    }

    /// Invokes any associated handlers for `input`.
    ///
    /// Returns whether the event has been handled or not.
    pub fn input(&mut self, input: KeyEvent) -> EventHandling {
        for modifiers in FuzzyModifiers(input.modifiers.state()) {
            let physical_match = self.0.get(&Shortcut {
                key: ShortcutKey::Physical(input.physical_key),
                modifiers,
            });
            let logical_match = self.0.get(&Shortcut {
                key: ShortcutKey::Logical(input.logical_key.clone()),
                modifiers,
            });
            match (physical_match, logical_match) {
                (Some(physical), Some(logical)) if physical.callback != logical.callback => {
                    if input.state.is_pressed() && (!input.repeat || physical.repeat) {
                        physical.callback.invoke(input.clone());
                    }
                    if input.state.is_pressed() && (!input.repeat || logical.repeat) {
                        logical.callback.invoke(input);
                    }
                    return HANDLED;
                }
                (Some(callback), _) | (_, Some(callback)) => {
                    if input.state.is_pressed() && (!input.repeat || callback.repeat) {
                        callback.callback.invoke(input);
                    }
                    return HANDLED;
                }
                _ => {}
            }
        }

        IGNORED
    }
}

/// An iterator that attempts one fallback towards a common shortcut modifier.
///
/// The precedence for the fallback is: Primary, Control, Super.
struct FuzzyModifiers(ModifiersState);

impl Iterator for FuzzyModifiers {
    type Item = ModifiersState;

    fn next(&mut self) -> Option<Self::Item> {
        let modifiers = self.0;
        if modifiers.is_empty() {
            return None;
        } else if modifiers.primary() && !modifiers.only_primary() {
            self.0 = ModifiersState::PRIMARY;
        } else if modifiers.control_key() && !modifiers.only_control() {
            self.0 = ModifiersState::CONTROL;
        } else if modifiers.super_key() && !modifiers.only_super() {
            self.0 = ModifiersState::SUPER;
        } else {
            self.0 = ModifiersState::empty();
        }
        Some(modifiers)
    }
}

/// A widget that handles keyboard shortcuts.
#[derive(Debug)]
pub struct Shortcuts {
    shortcuts: ShortcutMap,
    child: WidgetRef,
}

impl Shortcuts {
    /// Wraps `child` with keyboard shortcut handling.
    #[must_use]
    pub fn new(child: impl MakeWidget) -> Self {
        Self {
            shortcuts: ShortcutMap::default(),
            child: WidgetRef::new(child),
        }
    }

    /// Invokes `callback` when `key` is pressed while `modifiers` are pressed.
    ///
    /// This shortcut will only be invoked if focus is within a child of this
    /// widget, or if this widget becomes the root widget of a window.
    #[must_use]
    pub fn with_shortcut<F>(
        mut self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) -> Self
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.shortcuts.insert(key, modifiers, callback);
        self
    }

    /// Invokes `callback` when `key` is pressed while `modifiers` are pressed.
    /// If the shortcut is held, the callback will be invoked on repeat events.
    ///
    /// This shortcut will only be invoked if focus is within a child of this
    /// widget, or if this widget becomes the root widget of a window.
    #[must_use]
    pub fn with_repeating_shortcut<F>(
        mut self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) -> Self
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.shortcuts.insert_repeating(key, modifiers, callback);
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Shortcut {
    pub key: ShortcutKey,
    pub modifiers: ModifiersState,
}

impl Shortcut {
    fn into_variations(self) -> (Shortcut, Option<Shortcut>) {
        let modifiers = self.modifiers;
        let extra = match &self.key {
            ShortcutKey::Logical(Key::Character(c)) => {
                let lowercase = SmolStr::new(c.to_lowercase());
                let uppercase = SmolStr::new(c.to_uppercase());
                if c == &lowercase {
                    Some(Shortcut {
                        key: uppercase.into(),
                        modifiers,
                    })
                } else {
                    Some(Shortcut {
                        key: lowercase.into(),
                        modifiers,
                    })
                }
            }
            _ => None,
        };
        (self, extra)
    }
}

impl From<PhysicalKey> for ShortcutKey {
    fn from(key: PhysicalKey) -> Self {
        ShortcutKey::Physical(key)
    }
}

impl From<Key> for ShortcutKey {
    fn from(key: Key) -> Self {
        ShortcutKey::Logical(key)
    }
}

impl From<NamedKey> for ShortcutKey {
    fn from(key: NamedKey) -> Self {
        Self::from(Key::from(key))
    }
}

impl From<NativeKey> for ShortcutKey {
    fn from(key: NativeKey) -> Self {
        Self::from(Key::from(key))
    }
}

impl From<SmolStr> for ShortcutKey {
    fn from(key: SmolStr) -> Self {
        Self::from(Key::Character(key))
    }
}

impl From<&'_ str> for ShortcutKey {
    fn from(key: &'_ str) -> Self {
        Self::from(SmolStr::new(key))
    }
}

impl From<KeyCode> for ShortcutKey {
    fn from(key: KeyCode) -> Self {
        Self::from(PhysicalKey::from(key))
    }
}

impl From<NativeKeyCode> for ShortcutKey {
    fn from(key: NativeKeyCode) -> Self {
        Self::from(PhysicalKey::from(key))
    }
}

#[derive(Debug, Clone)]
struct ShortcutConfig {
    repeat: bool,
    callback: SharedCallback<KeyEvent, EventHandling>,
}

/// A key used in a [`Shortcuts`] widget.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ShortcutKey {
    /// A logical key.
    ///
    /// Logical keys are mapped using the operating system configuration.
    Logical(Key),

    /// A physical key.
    ///
    /// Physical keys represent a physical keyboard location and may be
    /// different logical keys depending on operating system configurations.
    Physical(PhysicalKey),
}

impl WrapperWidget for Shortcuts {
    fn child_mut(&mut self) -> &mut crate::widget::WidgetRef {
        &mut self.child
    }

    fn keyboard_input(
        &mut self,
        _device_id: crate::window::DeviceId,
        input: KeyEvent,
        _is_synthetic: bool,
        _context: &mut crate::context::EventContext<'_>,
    ) -> EventHandling {
        self.shortcuts.input(input)
    }
}
