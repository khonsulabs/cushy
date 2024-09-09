//! A keyboard shortcut handling widget.

use ahash::AHashMap;
use kludgine::app::winit::keyboard::{
    Key, KeyCode, ModifiersState, NamedKey, NativeKey, NativeKeyCode, PhysicalKey, SmolStr,
};

use crate::widget::{
    EventHandling, MakeWidget, SharedCallback, WidgetRef, WrapperWidget, HANDLED, IGNORED,
};
use crate::window::KeyEvent;

/// A widget that handles keyboard shortcuts.
#[derive(Debug)]
pub struct Shortcuts {
    shortcuts: AHashMap<Shortcut, ShortcutConfig>,
    child: WidgetRef,
}

impl Shortcuts {
    /// Wraps `child` with keyboard shortcut handling.
    #[must_use]
    pub fn new(child: impl MakeWidget) -> Self {
        Self {
            shortcuts: AHashMap::new(),
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
        self.insert_shortcut(key.into(), modifiers, false, SharedCallback::new(callback));
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
        self.insert_shortcut(key.into(), modifiers, true, SharedCallback::new(callback));
        self
    }

    fn insert_shortcut(
        &mut self,
        key: ShortcutKey,
        modifiers: ModifiersState,
        repeat: bool,
        callback: SharedCallback<KeyEvent, EventHandling>,
    ) {
        let (first, second) = Shortcut { key, modifiers }.into_variations();
        let config = ShortcutConfig { repeat, callback };

        if let Some(second) = second {
            self.shortcuts.insert(second, config.clone());
        }

        self.shortcuts.insert(first, config);
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
        let physical_match = self.shortcuts.get(&Shortcut {
            key: ShortcutKey::Physical(input.physical_key),
            modifiers: input.modifiers.state(),
        });
        let logical_match = self.shortcuts.get(&Shortcut {
            key: ShortcutKey::Logical(input.logical_key.clone()),
            modifiers: input.modifiers.state(),
        });
        match (physical_match, logical_match) {
            (Some(physical), Some(logical)) if physical.callback != logical.callback => {
                if input.state.is_pressed() && (!input.repeat || physical.repeat) {
                    physical.callback.invoke(input.clone());
                }
                if input.state.is_pressed() && (!input.repeat || logical.repeat) {
                    logical.callback.invoke(input);
                }
                HANDLED
            }
            (Some(callback), _) | (_, Some(callback)) => {
                if input.state.is_pressed() && (!input.repeat || callback.repeat) {
                    callback.callback.invoke(input);
                }
                HANDLED
            }
            _ => IGNORED,
        }
    }
}
