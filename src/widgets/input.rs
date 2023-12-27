//! A text input widget.

use std::borrow::{Borrow, BorrowMut, Cow};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter, Write};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use intentional::Cast;
use kludgine::app::winit::event::{ElementState, Ime, KeyEvent};
use kludgine::app::winit::keyboard::{Key, NamedKey};
use kludgine::app::winit::window::{CursorIcon, ImePurpose};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{
    Abs, FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, Round, ScreenScale, Size,
};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::{CanRenderTo, Color, DrawableExt};
use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};
use zeroize::Zeroizing;

use crate::context::{EventContext, GraphicsContext, LayoutContext};
use crate::styles::components::{HighlightColor, IntrinsicPadding, OutlineColor, TextColor};
use crate::utils::ModifiersExt;
use crate::value::{Dynamic, Generation, IntoDynamic, IntoValue, Value};
use crate::widget::{Callback, EventHandling, Widget, HANDLED, IGNORED};
use crate::{ConstraintLimit, Lazy};

const CURSOR_BLINK_DURATION: Duration = Duration::from_millis(500);

/// A text input widget.
#[must_use]
pub struct Input<Storage> {
    /// The value of this widget.
    pub value: Dynamic<Storage>,
    /// The placeholder text to display when no value is present.
    pub placeholder: Value<String>,
    mask_symbol: Value<CowString>,
    mask: CowString,
    on_key: Option<Callback<KeyEvent, EventHandling>>,
    cache: Option<CachedLayout>,
    selection: SelectionState,
    blink_state: BlinkState,
    needs_to_select_all: bool,
    mouse_buttons_down: usize,
    line_navigation_x_target: Option<Px>,
    window_focused: bool,
}

#[derive(Eq, PartialEq, Clone, Copy)]
struct CacheKey {
    generation: Generation,
    mask_generation: Option<Generation>,
    placeholder_generation: Option<Generation>,
    width: Option<Px>,
    color: Color,
    mask_bytes: usize,
    cursor: Cursor,
    selection: Option<Cursor>,
}

struct CachedLayout {
    bytes: usize,
    measured: MeasuredText<Px>,
    placeholder: MeasuredText<Px>,
    key: CacheKey,
}

/// The current selection of an [`Input`].
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct SelectionState {
    /// The cursor location, which is what is moved when the user types or uses
    /// the arrow keys.
    pub cursor: Cursor,
    /// The start of the selection, which is the original cursor location when
    /// the current series of selection actions began.
    pub start: Option<Cursor>,
}

/// A location within an [`Input`] widget.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Ord, PartialOrd, Default)]
pub struct Cursor {
    /// A byte offset within the value of the [`Input`] widget.
    pub offset: usize,
    /// The direction the cursor should be placed relative to the line end.
    pub affinity: Affinity,
}

/// An affinity towards a direction.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Ord, PartialOrd, Default)]
pub enum Affinity {
    /// The affinity is before the item in question.
    #[default]
    Before,
    /// The affinity is after the item in question.
    After,
}

impl<Storage> Input<Storage>
where
    Storage: InputStorage,
{
    /// Returns a new widget containing `initial_text`.
    pub fn new(initial_value: impl IntoDynamic<Storage>) -> Self {
        Self {
            value: initial_value.into_dynamic(),
            mask: CowString::default(),
            mask_symbol: Storage::MASKED
                .then(|| CowString::from('\u{2022}'))
                .unwrap_or_default()
                .into_value(),
            placeholder: Value::default(),
            cache: None,
            blink_state: BlinkState::default(),
            selection: SelectionState::default(),
            on_key: None,
            mouse_buttons_down: 0,
            needs_to_select_all: true,
            line_navigation_x_target: None,
            window_focused: false,
        }
    }

    /// Sets the `placeholder` text, which is displayed when the field has an
    /// empty value.
    pub fn placeholder(mut self, placeholder: impl IntoValue<String>) -> Self {
        self.placeholder = placeholder.into_value();
        self
    }

    /// Sets the symbol to use for masking sensitive content to `symbol`.
    ///
    /// Only the first unicode grapheme will be used for the symbol. A warning
    /// will be printed if a multi-grapheme string is provided.
    ///
    /// When using a [`InputStorage`] that is masked by default, the unicode
    /// bullet character (`\u{2022}`) is used as the default.
    pub fn mask_symbol(mut self, symbol: impl IntoValue<CowString>) -> Self {
        self.mask_symbol = symbol.into_value();

        self
    }

    /// Sets the `on_key` callback.
    ///
    /// This function is called for every keyboard input event. If [`HANDLED`]
    /// is returned, this widget will ignore the event.
    pub fn on_key<F>(mut self, on_key: F) -> Self
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        self.on_key = Some(Callback::new(on_key));
        self
    }

    fn select_all(&mut self) {
        self.value.map_ref(|value| {
            let text = value.as_str();

            self.selection.start = Some(Cursor::default());
            self.selection.cursor.offset = text.len();
            self.selection.cursor.affinity = Affinity::After;
        });
    }

    fn forward_delete(&mut self, context: &mut EventContext<'_, '_>) {
        if !context.enabled() {
            return;
        }

        let (cursor, selection) = self.selected_range();
        if let Some(selection) = selection {
            self.replace_range(cursor, selection, "");
        } else {
            let mut value = self.value.lock();
            if let Some(length) = value.as_str()[cursor.offset..]
                .graphemes(true)
                .next()
                .map(str::len)
            {
                value
                    .as_string_mut()
                    .replace_range(cursor.offset..cursor.offset + length, "");
            }
        }
    }

    fn replace_range(&mut self, start: Cursor, end: Cursor, new_text: &str) {
        self.value.map_mut(|value| {
            let value = value.as_string_mut();
            let start = start.offset.min(value.len().saturating_sub(1));
            let end = end.offset.min(value.len());
            value.replace_range(start..end, new_text);

            self.selection.cursor.offset = start + new_text.len();
            self.selection.start = None;
        });
    }

    fn delete(&mut self, context: &mut EventContext<'_, '_>) {
        if !context.enabled() {
            return;
        }

        let (cursor, selection) = self.selected_range();
        if let Some(selection) = selection {
            self.replace_range(cursor, selection, "");
        } else if cursor.offset > 0 {
            let mut value = self.value.lock();
            let length = value.as_str().len();

            if length == 0 || cursor.offset == 0 {
                return;
            }

            if let Ok(Some(offset)) = GraphemeCursor::new(cursor.offset, value.as_str().len(), true)
                .prev_boundary(value.as_str(), 0)
            {
                value
                    .as_string_mut()
                    .replace_range(offset..cursor.offset, "");
                self.selection.cursor.offset -= cursor.offset - offset;
            }
        }
    }

    fn move_cursor(
        &mut self,
        direction: Affinity,
        mode: CursorNavigationMode,
        context: &mut EventContext<'_, '_>,
    ) {
        if !matches!(mode, CursorNavigationMode::Line) {
            self.line_navigation_x_target = None;
        }

        // @ecton: After a lot of thought, it seems like the only way for
        // affinity to be switched to After is via dragging the mouse.
        self.selection.cursor.affinity = Affinity::Before;
        match mode {
            CursorNavigationMode::Grapheme => self.move_cursor_by_grapheme(direction),
            CursorNavigationMode::Word => self.move_cursor_by_word(direction),
            CursorNavigationMode::Line => self.move_cursor_by_line(direction, context),
            CursorNavigationMode::LineExtent => self.move_cursor_by_line_extent(direction, context),
        }
    }

    fn move_cursor_by_grapheme(&mut self, affinity: Affinity) {
        let value = self.value.lock();
        let length = value.as_str().len();

        match affinity {
            Affinity::Before => {
                if let Some((_, grapheme)) =
                    value
                        .as_str()
                        .grapheme_indices(true)
                        .find(|(index, grapheme)| {
                            index + grapheme.len() == self.selection.cursor.offset
                        })
                {
                    self.selection.cursor.offset -= grapheme.len();
                } else {
                    self.selection.cursor.offset = 0;
                }
            }
            Affinity::After => {
                if self.selection.cursor.offset < length {
                    if let Some(grapheme) = value.as_str()[self.selection.cursor.offset..]
                        .graphemes(true)
                        .next()
                    {
                        self.selection.cursor.offset += grapheme.len();
                    } else {
                        self.selection.cursor.offset = length;
                    }
                }
            }
        }
    }

    fn move_cursor_by_word(&mut self, affinity: Affinity) {
        let value = self.value.lock();
        let length = value.as_str().len();
        match affinity {
            Affinity::Before => {
                let mut words = value.as_str().unicode_word_indices().peekable();
                while let Some((index, _)) = words.next() {
                    let next_starts_after_selection = words
                        .peek()
                        .map_or(true, |(index, _)| *index >= self.selection.cursor.offset);
                    if next_starts_after_selection {
                        self.selection.cursor.offset = index;
                        return;
                    }
                }

                self.selection.cursor.offset = 0;
            }
            Affinity::After => {
                if self.selection.cursor.offset < length {
                    if let Some((index, word)) = value.as_str()[self.selection.cursor.offset..]
                        .unicode_word_indices()
                        .next()
                    {
                        self.selection.cursor.offset += index + word.len();
                    } else {
                        self.selection.cursor.offset = length;
                    }
                }
            }
        }
    }

    fn move_cursor_by_line_extent(
        &mut self,
        affinity: Affinity,
        context: &mut EventContext<'_, '_>,
    ) {
        let Some(cache) = self.cache.as_ref() else {
            return;
        };

        let (mut position, _) = self.point_from_cursor(cache, self.selection.cursor, cache.bytes);
        position.y += context
            .get(&IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        match affinity {
            Affinity::Before => position.x = Px::ZERO,
            Affinity::After => {
                position.x = context.last_layout().map_or(Px::MAX, |r| r.size.width);
            }
        };

        self.selection.cursor = self.cursor_from_point(position, context);
    }

    fn move_cursor_by_line(&mut self, affinity: Affinity, context: &mut EventContext<'_, '_>) {
        let Some(cache) = self.cache.as_ref() else {
            return;
        };

        let (mut position, _) = self.point_from_cursor(cache, self.selection.cursor, cache.bytes);
        position += Point::squared(
            context
                .get(&IntrinsicPadding)
                .into_px(context.kludgine.scale())
                .round(),
        );
        if let Some(target_x) = self.line_navigation_x_target {
            position.x = target_x;
        } else {
            self.line_navigation_x_target = Some(position.x);
        }
        match affinity {
            Affinity::Before => position.y -= cache.measured.line_height,
            Affinity::After => {
                position.y += cache.measured.line_height;
            }
        };

        self.selection.cursor = self.cursor_from_point(position, context);
    }

    fn constrain_selection(&mut self) {
        let length = self.value.map_ref(|s| s.as_str().len());
        self.selection.cursor.offset = self.selection.cursor.offset.min(length);
        if let Some(start) = &mut self.selection.start {
            start.offset = start.offset.min(length);
        }
    }

    fn selected_range(&mut self) -> (Cursor, Option<Cursor>) {
        self.constrain_selection();
        match self.selection.start {
            Some(start) => match start.offset.cmp(&self.selection.cursor.offset) {
                Ordering::Less => (start, Some(self.selection.cursor)),
                Ordering::Equal => {
                    if self.mouse_buttons_down == 0 {
                        self.selection.start = None;
                    }
                    (self.selection.cursor, None)
                }
                Ordering::Greater => (self.selection.cursor, Some(start)),
            },
            None => (self.selection.cursor, None),
        }
    }

    fn map_selected_text<R>(&mut self, map: impl FnOnce(&str) -> R) -> Option<R> {
        let (cursor, Some(end)) = self.selected_range() else {
            return None;
        };

        Some(
            self.value
                .map_ref(|value| map(&value.as_str()[cursor.offset..end.offset])),
        )
    }

    fn is_masked(&self) -> bool {
        self.mask_symbol.map(|mask| !mask.is_empty())
    }

    fn copy_selection_to_clipboard(&mut self, context: &mut EventContext<'_, '_>) {
        if self.is_masked() {
            return;
        }

        self.map_selected_text(|text| {
            if let Some(mut clipboard) = context.clipboard_guard() {
                match clipboard.set_text(text) {
                    Ok(()) => {}
                    Err(err) => tracing::error!("error copying to clipboard: {err}"),
                }
            }
        });
    }

    fn replace_selection(&mut self, new_text: &str, context: &mut EventContext<'_, '_>) {
        if !context.enabled() {
            return;
        }

        let selected_range = self.selected_range();
        match selected_range {
            (start, Some(end)) => {
                self.replace_range(start, end, new_text);
            }
            (cursor, None) => {
                let mut value = self.value.lock();
                if cursor.offset < value.as_str().len() {
                    value.as_string_mut().insert_str(cursor.offset, new_text);
                    self.selection.cursor.offset += new_text.len();
                } else {
                    value.as_string_mut().push_str(new_text);
                    self.selection.cursor.offset += new_text.len();
                }
            }
        };
    }

    fn paste_from_clipboard(&mut self, context: &mut EventContext<'_, '_>) -> bool {
        if !context.enabled() {
            return false;
        }

        match context
            .clipboard_guard()
            .map(|mut clipboard| clipboard.get_text())
        {
            Some(Ok(text)) => {
                self.replace_selection(&text, context);
                true
            }
            None | Some(Err(arboard::Error::ConversionFailure)) => false,
            Some(Err(err)) => {
                tracing::error!("error retrieving clipboard contents: {err}");
                false
            }
        }
    }

    fn handle_key(&mut self, input: KeyEvent, context: &mut EventContext<'_, '_>) -> EventHandling {
        match (input.state, input.logical_key, input.text.as_deref()) {
            (ElementState::Pressed,  Key::Named(key @ (NamedKey::Backspace| NamedKey::Delete)), _) => {
                match key {
                    NamedKey::Backspace => self.delete(context),
                    NamedKey::Delete => self.forward_delete(context),
                    _ => unreachable!("previously matched"),
                }

                HANDLED
            }
            (ElementState::Pressed, Key::Named(key @ (NamedKey::ArrowLeft | NamedKey::ArrowDown | NamedKey::ArrowUp | NamedKey::ArrowRight | NamedKey::Home | NamedKey::End)), _) => {
                let modifiers = context.modifiers();
                let affinity = if matches!(key, NamedKey::ArrowLeft | NamedKey::ArrowUp | NamedKey::Home) {
                    Affinity::Before
                } else {
                    Affinity::After
                };
                match (self.selection.start, modifiers.state().shift_key()) {
                    (None, true) => {
                        self.selection.start = Some(self.selection.cursor);
                    }
                    (Some(start), false) => {
                        self.selection.cursor = if affinity == Affinity::Before {
                            start.min(self.selection.cursor)
                        } else {
                            start.max(self.selection.cursor)
                        };
                        self.selection.start = None;
                    }
                    _ => {}
                };

                match key {
                    #[cfg(any(target_os = "ios", target_os = "macos"))]
                    NamedKey::ArrowLeft | NamedKey::ArrowRight if modifiers.primary() => self.move_cursor(affinity, CursorNavigationMode::LineExtent, context),
                    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
                    NamedKey::Home | NamedKey::End => self.move_cursor(affinity, CursorNavigationMode::LineExtent, context),
                    NamedKey::ArrowLeft | NamedKey::ArrowRight if modifiers.word_select() => self.move_cursor(affinity, CursorNavigationMode::Word, context),
                    NamedKey::ArrowLeft | NamedKey::ArrowRight => self.move_cursor(affinity, CursorNavigationMode::Grapheme, context),
                    NamedKey::ArrowDown | NamedKey::ArrowUp => self.move_cursor(affinity, CursorNavigationMode::Line, context),
                    _ => tracing::warn!("unhandled key: {key:?}"),
                }

                HANDLED
            }
            (state, _, Some("a")) if context.modifiers().primary() => {
                if state.is_pressed() {
                    self.select_all();
                }
                HANDLED
            }
            (state, _, Some("c")) if context.modifiers().primary() => {

                if state.is_pressed() {
                    self.copy_selection_to_clipboard(context);
                }
                HANDLED
            }
            (state, _, Some("v")) if context.modifiers().primary() => {
                if state.is_pressed() {
                    self.paste_from_clipboard(context);
                }

                HANDLED
            }
            (state, _, Some(text))
                if !context.modifiers().primary()
                    && text != "\t" // tab
                    && text != "\r" // enter/return
                    && text != "\u{1b}" // escape
                    =>
            {
                if state.is_pressed() {
                    self.replace_selection(text, context);
                }
                HANDLED
            }
            (_, _, _) =>  IGNORED,
        }
    }

    fn layout_text(
        &mut self,
        width: Option<Px>,
        context: &mut GraphicsContext<'_, '_, '_, '_, '_>,
    ) {
        context.invalidate_when_changed(&self.value);

        let mut key = {
            let (cursor, selection) = self.selected_range();
            CacheKey {
                generation: self.value.generation(),
                mask_generation: self.mask_symbol.generation(),
                placeholder_generation: self.placeholder.generation(),
                width,
                color: context.get(&TextColor),
                mask_bytes: self
                    .mask_symbol
                    .map(|sym| sym.graphemes(true).next().map_or(0, str::len)),
                cursor,
                selection,
            }
        };
        match &mut self.cache {
            Some(cache)
                if cache.measured.can_render_to(&context.gfx)
                    && cache.placeholder.can_render_to(&context.gfx)
                    && cache.key == key => {}
            _ => {
                let (bytes, measured, placeholder, ) = self.value.map_ref(|storage| {
                    let mut text = storage.as_str();
                    let mut bytes = text.len();

                    self.mask_symbol.map(|mask_symbol| {
                        // Another thread could have updated the mask symbol
                        // since we checked above.
                        if let Some(first_grapheme) = mask_symbol.graphemes(true).next() {
                            if mask_symbol != first_grapheme {
                                static WARNING: OnceLock<()> = OnceLock::new();
                                WARNING.get_or_init(|| tracing::warn!("Mask symbol {mask_symbol} as more than one grapheme. Only the first grapheme will be used."));
                            }
                            // Technically something more optimal than asking the
                            // layout system to lay out a repeated string should be
                            // doable, but it seems like a lot of code.
                            key.mask_bytes = first_grapheme.len();
                            let char_count = text.graphemes(true).count();
                            bytes = key.mask_bytes * char_count;
                            self.mask.truncate(bytes);

                            while self.mask.len() < bytes {
                                self.mask.push_str(first_grapheme);
                            }
                            text = &self.mask;
                        } else {
                            key.mask_bytes = 0;
                        }
                    });

                    context.apply_current_font_settings();
                    let mut text = Text::new(text, key.color);
                    if let Some(width) = width {
                        text = text.wrap_at(width);
                    }

                    let placeholder_color = context.theme().surface.on_color_variant;
                    let placeholder = self.placeholder.map(|placeholder| context.gfx.measure_text(Text::new(placeholder, placeholder_color)));
                    (bytes, context.gfx.measure_text(text), placeholder)
                });
                self.cache = Some(CachedLayout {
                    bytes,
                    measured,
                    placeholder,
                    key,
                });
            }
        }
    }

    fn cache_info(&self) -> CacheInfo<'_> {
        let cache = self
            .cache
            .as_ref()
            .expect("always called after layout_text");

        // Adjust the selection cursors to accommodate the difference in unicode
        // widths of characters in the source string and the mask_char.

        let masked = cache.key.mask_bytes > 0;
        let mut cursor = cache.key.cursor;
        let mut selection = cache.key.selection;
        if masked {
            self.value.map_ref(|value| {
                let value = value.as_str();
                assert!(cache.key.cursor.offset <= value.len());
                cursor.offset =
                    value[..cache.key.cursor.offset].graphemes(true).count() * cache.key.mask_bytes;
                if let Some(selection) = &mut selection {
                    assert!(selection.offset <= value.len());
                    selection.offset =
                        value[..selection.offset].graphemes(true).count() * cache.key.mask_bytes;
                }
            });
        }

        CacheInfo {
            cache,
            masked,
            cursor,
            selection,
        }
    }

    #[allow(clippy::too_many_lines)] // it's text layout, c'mon
    fn point_from_cursor(
        &self,
        cache: &CachedLayout,
        cursor: Cursor,
        total_bytes: usize,
    ) -> (Point<Px>, Px) {
        if cache.measured.glyphs.is_empty()
            || (cursor.offset == 0 && cursor.affinity == Affinity::Before)
        {
            return (Point::default(), Px::ZERO);
        }

        // Space between glyphs isn't represented in the glyphs. If the cursor rests
        // within characters that have no glyphs (whitespace), we need to
        // approximate the position based on the location of the nearest glyphs.
        let mut closest_before_index = 0;
        let mut closest_after_index = usize::MAX;
        let mut bottom_right_index = 0;
        let mut bottom_right_line = 0;
        let mut bottom_right_rect = Rect::default();
        let mut unrendered_offset = 0;
        for (index, glyph) in cache.measured.glyphs.iter().enumerate() {
            unrendered_offset = unrendered_offset.max(glyph.info.end);
            let rect = glyph.rect();
            if bottom_right_rect.size.width == 0
                || glyph.info.line > bottom_right_line
                || (glyph.info.line == bottom_right_line
                    && rect.origin.x > bottom_right_rect.origin.x)
            {
                bottom_right_line = glyph.info.line;
                bottom_right_index = index;
                bottom_right_rect = rect;
            }

            match (
                glyph.info.start.cmp(&cursor.offset),
                cursor.offset.cmp(&glyph.info.end),
            ) {
                (Ordering::Less | Ordering::Equal, Ordering::Less) => {
                    // cosmic text may have grouped multiple graphemes into a single glyph.
                    let mut grapheme_offset = Px::ZERO;
                    if glyph.info.start < cursor.offset {
                        let clustered_bytes = glyph.info.end - glyph.info.start;
                        if clustered_bytes > 1 {
                            let clustered_graphemes = if cache.key.mask_bytes > 0 {
                                self.mask[glyph.info.start..glyph.info.end]
                                    .graphemes(true)
                                    .count()
                            } else {
                                self.value.map_ref(|value| {
                                    value.as_str()[glyph.info.start..glyph.info.end]
                                        .graphemes(true)
                                        .count()
                                })
                            };
                            if clustered_graphemes > 1 {
                                let cursor_offset = cursor.offset - glyph.info.start;

                                grapheme_offset = rect.size.width * cursor_offset.cast::<f32>()
                                    / clustered_graphemes.cast::<f32>();
                            }
                        }
                    }

                    return (
                        Point::new(
                            rect.origin.x + grapheme_offset,
                            cache.measured.line_height.saturating_mul(Px::new(
                                i32::try_from(glyph.info.line).unwrap_or(i32::MAX),
                            )),
                        ),
                        rect.size.width,
                    );
                }
                (Ordering::Less, _) => {
                    closest_before_index = closest_before_index.max(index);
                }
                (_, Ordering::Less) => {
                    closest_after_index = closest_after_index.min(index);
                }
                _ => {}
            }
        }

        if closest_after_index == usize::MAX {
            let bottom_right = &cache.measured.glyphs[bottom_right_index];
            let bottom_y = cache.measured.line_height.saturating_mul(Px::new(
                i32::try_from(bottom_right.info.line).unwrap_or(i32::MAX),
            ));
            // No glyph could be found that started/contained the cursors offset.
            let mut bottom_right_cursor = Point::new(
                bottom_right_rect.origin.x + bottom_right_rect.size.width,
                bottom_y,
            );
            let bytes_after_glyph = total_bytes - unrendered_offset;
            if !(bottom_right.info.end == cursor.offset || bytes_after_glyph == 0) {
                // We're rendering past the end of the text. We shuld probably try to
                // estimate the amount of whitespace should be visible based on the
                // number of whitespace characters at the end of the text.
                let space_past_glyph = bottom_right.info.line_width - bottom_right_cursor.x;
                let space_per_byte =
                    space_past_glyph.into_float() / bytes_after_glyph.cast::<f32>();
                let cursor_position = space_per_byte
                    * (cursor.offset.saturating_sub(unrendered_offset)).cast::<f32>();

                bottom_right_cursor.x += Px::from(cursor_position);
            }

            // The cursor should be placed after the bottom_right glyph
            (bottom_right_cursor, Px::ZERO)
        } else {
            let before = &cache.measured.glyphs[closest_before_index];
            let after = &cache.measured.glyphs[closest_after_index];
            let before_rect = before.rect();
            let after_rect = after.rect();
            let before_y = cache
                .measured
                .line_height
                .saturating_mul(Px::new(i32::try_from(before.info.line).unwrap_or(i32::MAX)));

            if before.info.line == after.info.line {
                let before_right = before_rect.origin.x + before_rect.size.width;
                let space_between = after_rect.origin.x - before_right;
                let bytes_between = after.info.start - before.info.end;
                let space_per_byte = space_between.into_float() / bytes_between.cast::<f32>();
                let cursor_position =
                    space_per_byte * (cursor.offset - before.info.end).cast::<f32>();

                (
                    Point::new(before_right + Px::from(cursor_position), before_y),
                    Px::from(space_per_byte),
                )
            } else {
                match cursor.affinity {
                    Affinity::Before => {
                        // TODO We need to look out for whitespace at the end of the line.
                        let mut origin = before_rect.origin;
                        origin.x += before_rect.size.width;
                        (origin, before_y)
                    }
                    Affinity::After => (
                        Point::new(Px::ZERO, before_y + cache.measured.line_height),
                        Px::ZERO,
                    ),
                }
            }
        }
    }

    fn cursor_from_point(
        &mut self,
        location: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> Cursor {
        let mut cursor = self.cached_cursor_from_point(location, context);
        if let Some(symbol) = self.mask.graphemes(true).next() {
            let grapheme_offset = cursor.offset / symbol.len();
            cursor.offset = self.value.map_ref(|value| {
                value
                    .as_str()
                    .graphemes(true)
                    .take(grapheme_offset)
                    .map(str::len)
                    .sum::<usize>()
            });
        }
        cursor
    }

    fn cached_cursor_from_point(
        &mut self,
        location: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> Cursor {
        let Some(cache) = &self.cache else {
            return Cursor::default();
        };

        let padding = context
            .get(&IntrinsicPadding)
            .into_px(context.kludgine.scale())
            .round();
        let mut location = location - padding;
        if location.y < 0 {
            location.y = Px::ZERO;
        }
        if location.x < 0 {
            location.x = Px::ZERO;
        }

        let mut closest: Option<(Cursor, i32, usize, Point<Px>)> = None;
        let mut current_line = usize::MAX;
        let mut current_line_y = Px::ZERO;
        for (index, glyph) in cache.measured.glyphs.iter().enumerate() {
            if current_line != glyph.info.line {
                current_line = glyph.info.line;

                current_line_y = cache
                    .measured
                    .line_height
                    .saturating_mul(Px::new(i32::try_from(current_line).unwrap_or(i32::MAX)));
            }
            let mut rect = glyph.rect();
            if !glyph.visible() {
                rect.size.height = cache.measured.line_height;
            }
            let relative = location - Point::new(rect.origin.x, current_line_y);
            if relative.x >= 0
                && relative.y >= 0
                && relative.x <= rect.size.width
                && relative.y <= cache.measured.line_height
            {
                return if relative.x > rect.size.width / 2 {
                    if glyph.info.end < cache.bytes {
                        Cursor {
                            offset: glyph.info.end,
                            affinity: Affinity::Before,
                        }
                    } else {
                        Cursor {
                            offset: glyph.info.start,
                            affinity: Affinity::After,
                        }
                    }
                } else {
                    Cursor {
                        offset: glyph.info.start,
                        affinity: Affinity::Before,
                    }
                };
            }

            // Make relative be relative to the center of the glyph for a nearest search.
            let relative = relative + rect.size / 2;

            let line_height = cache.measured.line_height.get();
            let xy = relative
                .x
                .get()
                .saturating_mul(
                    ((relative.y.get() + line_height - 1) / line_height * line_height)
                        .saturating_pow(2),
                )
                .saturating_abs();
            let cursor = Cursor {
                offset: if relative.x < 0 || relative.y < 0 {
                    glyph.info.start
                } else {
                    glyph.info.end
                },
                affinity: Affinity::Before,
            };
            match closest {
                Some((_, closest_xy, ..)) if xy < closest_xy => {
                    closest = Some((cursor, xy, index, relative));
                }
                None => closest = Some((cursor, xy, index, relative)),
                _ => {}
            }
        }

        if let Some((closest, _, index, relative)) = closest {
            // Having whitespace not in the measured text is really annoying.
            // This trick only works for the first line of text. Maybe we should
            // try and create a structure that organizes the glyphs into lines
            // so that it's easier to inspect and detect when there's
            // whitespace. For now, this is just a hack that helps get *some*
            // selection at the end of the input for trailing whitespace.
            if relative.x.abs() < cache.measured.line_height && index < cache.measured.glyphs.len()
            {
                return closest;
            }
        }

        Cursor {
            offset: cache.bytes,
            affinity: Affinity::After,
        }
    }
}

struct CacheInfo<'a> {
    cache: &'a CachedLayout,
    masked: bool,
    cursor: Cursor,
    selection: Option<Cursor>,
}

#[derive(Debug, Clone, Copy)]
enum CursorNavigationMode {
    Grapheme,
    Word,
    LineExtent,
    Line,
    // Document,
}

impl<Storage> Debug for Input<Storage>
where
    Storage: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Input")
            .field("text", &self.value)
            .field("mask_symbol", &self.mask_symbol)
            .field("placeholder", &self.placeholder)
            .finish_non_exhaustive()
    }
}

impl<Storage> Widget for Input<Storage>
where
    Storage: InputStorage + Debug,
{
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn accept_focus(&mut self, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: kludgine::app::winit::event::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        self.mouse_buttons_down += 1;
        context.focus();
        self.needs_to_select_all = false;
        self.selection.cursor = self.cursor_from_point(location, context);
        self.selection.start = Some(self.selection.cursor);
        context.set_needs_redraw();
        HANDLED
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        _context: &mut EventContext<'_, '_>,
    ) -> Option<CursorIcon> {
        Some(CursorIcon::Text)
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: kludgine::app::winit::event::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
        let cursor_location = self.cursor_from_point(location, context);
        if self.selection.cursor != cursor_location {
            self.selection.cursor = cursor_location;
            context.set_needs_redraw();
        }
        self.blink_state.force_on();
    }

    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: kludgine::app::winit::event::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        self.mouse_buttons_down -= 1;
    }

    #[allow(clippy::too_many_lines)]
    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>) {
        if self.needs_to_select_all {
            self.needs_to_select_all = false;
            self.select_all();
        }

        self.blink_state.update(context.elapsed());
        let window_focused = context.window().focused().get_tracking_redraw(context);
        if window_focused != self.window_focused {
            if window_focused {
                self.blink_state.force_on();
            }
            self.window_focused = window_focused;
        }

        let cursor_state = self.blink_state;
        let size = context.gfx.size();
        let padding = context
            .get(&IntrinsicPadding)
            .into_px(context.gfx.scale())
            .round();
        let padding = Point::squared(padding);

        self.layout_text(Some(size.width.into_signed()), context);
        let info = self.cache_info();

        let highlight = if context.focused(false) && window_focused {
            context.draw_focus_ring();
            context.get(&HighlightColor)
        } else {
            let outline_color = context.get(&OutlineColor);
            context.stroke_outline::<Lp>(outline_color, StrokeOptions::default());
            outline_color
        };

        if context.focused(false) {
            context.set_ime_allowed(true);
            context.set_ime_purpose(if info.masked {
                ImePurpose::Password
            } else {
                ImePurpose::Normal
            });

            if let Some(selection) = info.selection {
                let (start, end) = if selection < info.cursor {
                    (selection, info.cursor)
                } else {
                    (info.cursor, selection)
                };

                let (start_position, _) =
                    self.point_from_cursor(info.cache, start, info.cache.bytes);
                let (end_position, end_width) =
                    self.point_from_cursor(info.cache, end, info.cache.bytes);

                if start_position.y == end_position.y {
                    // Single line selection
                    let width = end_position.x - start_position.x;
                    context.gfx.draw_shape(
                        Shape::filled_rect(
                            Rect::new(
                                start_position,
                                Size::new(width, info.cache.measured.line_height),
                            ),
                            highlight,
                        )
                        .translate_by(padding),
                    );
                } else {
                    // Draw from start to end of line,
                    let width = size.width.into_signed() - start_position.x;
                    context.gfx.draw_shape(
                        Shape::filled_rect(
                            Rect::new(
                                start_position,
                                Size::new(width, info.cache.measured.line_height),
                            ),
                            highlight,
                        )
                        .translate_by(padding),
                    );
                    // Fill region between
                    let bottom_of_first_line = start_position.y + info.cache.measured.line_height;
                    let distance_between = end_position.y - bottom_of_first_line;
                    if distance_between > 0 {
                        context.gfx.draw_shape(
                            Shape::filled_rect(
                                Rect::new(
                                    Point::new(Px::ZERO, bottom_of_first_line),
                                    Size::new(size.width.into_signed(), distance_between),
                                ),
                                highlight,
                            )
                            .translate_by(padding),
                        );
                    }
                    // Draw from 0 to end + width
                    context.gfx.draw_shape(
                        Shape::filled_rect(
                            Rect::new(
                                Point::new(Px::ZERO, end_position.y),
                                Size::new(
                                    end_position.x + end_width,
                                    info.cache.measured.line_height,
                                ),
                            ),
                            highlight,
                        )
                        .translate_by(padding),
                    );
                }
            } else if window_focused && context.enabled() {
                let (location, _) =
                    self.point_from_cursor(info.cache, info.cursor, info.cache.bytes);
                if cursor_state.visible {
                    let cursor_width = Lp::points(2).into_px(context.gfx.scale());
                    context.gfx.draw_shape(
                        Shape::filled_rect(
                            Rect::new(
                                Point::new(location.x - cursor_width / 2, location.y),
                                Size::new(cursor_width, info.cache.measured.line_height),
                            ),
                            highlight,
                        )
                        .translate_by(padding),
                    );
                }
                context.redraw_in(cursor_state.remaining_until_blink);
            } else {
                context.redraw_when_changed(context.window().focused());
            }
        }

        let text = if info.cache.bytes > 0 {
            &info.cache.measured
        } else {
            &info.cache.placeholder
        };
        context
            .gfx
            .draw_measured_text(text.translate_by(padding), TextOrigin::TopLeft);
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let padding = context
            .get(&IntrinsicPadding)
            .into_upx(context.gfx.scale())
            .round();

        let width = available_space.width.max().saturating_sub(padding * 2);

        self.layout_text(Some(width.into_signed()), &mut context.graphics);
        let info = self.cache_info();

        info.cache
            .measured
            .size
            .max(info.cache.placeholder.size)
            .into_unsigned()
            + Size::squared(padding * 2)
    }

    fn keyboard_input(
        &mut self,
        _device_id: kludgine::app::winit::event::DeviceId,
        input: kludgine::app::winit::event::KeyEvent,
        _is_synthetic: bool,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        if let Some(on_key) = &mut self.on_key {
            on_key.invoke(input.clone())?;
        }

        let handled = self.handle_key(input, context);

        if handled.is_break() {
            context.set_needs_redraw();
        }

        self.blink_state.force_on();

        handled
    }

    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_, '_>) -> EventHandling {
        match ime {
            Ime::Enabled | Ime::Disabled => {}
            Ime::Preedit(text, cursor) => {
                tracing::warn!("TODO: preview IME input {text}, cursor: {cursor:?}");
            }
            Ime::Commit(text) => {
                self.replace_selection(&text, context);
                context.set_needs_redraw();
            }
        }

        HANDLED
    }

    fn focus(&mut self, context: &mut EventContext<'_, '_>) {
        if self.mouse_buttons_down == 0 {
            self.needs_to_select_all = true;
        }

        context.set_ime_allowed(true);
        context.set_ime_purpose(if self.is_masked() {
            ImePurpose::Password
        } else {
            ImePurpose::Normal
        });
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_, '_>) {
        context.set_ime_allowed(false);
        context.set_needs_redraw();
    }
}

#[derive(Debug, PartialEq, Eq)]
struct NotVisible(Point<Px>, usize);

#[derive(Clone, Copy)]
struct BlinkState {
    visible: bool,
    remaining_until_blink: Duration,
}

impl Default for BlinkState {
    fn default() -> Self {
        Self {
            visible: true,
            remaining_until_blink: CURSOR_BLINK_DURATION,
        }
    }
}

impl BlinkState {
    pub fn update(&mut self, elapsed: Duration) {
        let total_cycles = elapsed.as_nanos() / CURSOR_BLINK_DURATION.as_nanos();
        let remaining = Duration::from_nanos(
            u64::try_from(elapsed.as_nanos() % CURSOR_BLINK_DURATION.as_nanos())
                .expect("remainder fits in u64"),
        );
        // If we have an odd number of totaal cycles, flip the visibility.
        if total_cycles & 1 == 1 {
            self.visible = !self.visible;
        }

        if let Some(remaining) = self.remaining_until_blink.checked_sub(remaining) {
            self.remaining_until_blink = remaining;
        } else {
            self.visible = !self.visible;
            self.remaining_until_blink =
                CURSOR_BLINK_DURATION - (remaining - self.remaining_until_blink);
        }
    }

    pub fn force_on(&mut self) {
        self.visible = true;
        self.remaining_until_blink = CURSOR_BLINK_DURATION;
    }
}

/// A type that can be used as the storage of an [`Input`]'s string value.
///
/// This crate implements this trait for these types:
///
/// - [`String`]
/// - `Cow<'static, str>`
/// - [`CowString`]
/// - [`MaskedString`]
pub trait InputStorage: Send + 'static {
    /// If true, the input field should display a mask instead of the actual
    /// string by default.
    const MASKED: bool;

    /// Returns a reference to the contents as a `str`.
    fn as_str(&self) -> &str;
    /// Returns an exclusive reference to the contents as a `String`.
    fn as_string_mut(&mut self) -> &mut String;
}

impl InputStorage for String {
    const MASKED: bool = false;

    fn as_str(&self) -> &str {
        self.borrow()
    }

    fn as_string_mut(&mut self) -> &mut String {
        self.borrow_mut()
    }
}

impl InputStorage for Cow<'static, str> {
    const MASKED: bool = false;

    fn as_str(&self) -> &str {
        self.borrow()
    }

    fn as_string_mut(&mut self) -> &mut String {
        self.to_mut()
    }
}

/// A type that can be converted into a [`Dynamic`] containing `Storage`.
pub trait InputValue<Storage>: IntoDynamic<Storage> + Sized
where
    Storage: InputStorage,
{
    /// Returns this string as a text input widget.
    fn into_input(self) -> Input<Storage> {
        Input::new(self.into_dynamic())
    }
}

impl<T> InputValue<String> for T where T: IntoDynamic<String> {}
impl<T> InputValue<Cow<'static, str>> for T where T: IntoDynamic<Cow<'static, str>> {}

/// A cheap-to-clone, copy-on-write [`String`] type that implements
/// [`InputStorage`].
#[derive(Eq, Clone, Hash, Ord)]
pub struct CowString(Arc<String>);

impl CowString {
    /// Returns a new copy-on-write string with `str` as its contents.
    pub fn new(str: impl Into<String>) -> Self {
        Self(Arc::new(str.into()))
    }
}

impl Debug for CowString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for CowString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl<T> PartialOrd<T> for CowString
where
    T: PartialOrd<str> + ?Sized,
{
    fn partial_cmp(&self, other: &T) -> Option<Ordering> {
        other.partial_cmp(self.as_str()).map(Ordering::reverse)
    }
}

/// A cheap-to-clone, copy-on-write [`String`] type that masks its contents in
/// [`Debug`] and [`InputStorage`] implementations.
///
/// This type is designed to be used with an [`Input`] widget to create a
/// password/secure text entry field.
///
/// Internally, [`zeroize::Zeroizing`] is used to clear any contents of all
/// instances of [`MaskedString`] upon drop.
#[derive(Eq, Clone)]
pub struct MaskedString(Arc<Zeroizing<String>>);

impl MaskedString {
    /// Returns a new copy-on-write string with `str` as its contents.
    ///
    /// When used in an [`Input`] widget, the input will be masked by default.
    pub fn new(str: impl Into<String>) -> Self {
        Self(Arc::new(Zeroizing::new(str.into())))
    }
}

impl Debug for MaskedString {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.write_str("MaskedString(")?;
            for _ in 0..self.as_str().len() {
                f.write_char('*')?;
            }
            f.write_char(')')
        } else {
            f.debug_struct("MaskedString").finish_non_exhaustive()
        }
    }
}

macro_rules! impl_cow_string {
    ($type:ident, $masked:literal) => {
        impl Deref for $type {
            type Target = String;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $type {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut *Arc::make_mut(&mut self.0)
            }
        }

        impl Default for $type {
            fn default() -> Self {
                static EMPTY: Lazy<$type> = Lazy::new(|| $type(Arc::default()));
                EMPTY.clone()
            }
        }

        impl From<char> for $type {
            fn from(s: char) -> Self {
                Self::new(s)
            }
        }

        impl IntoValue<$type> for char {
            fn into_value(self) -> Value<$type> {
                Value::Constant(<$type>::from(self))
            }
        }

        impl From<String> for $type {
            fn from(s: String) -> Self {
                Self::new(s)
            }
        }

        impl IntoValue<$type> for String {
            fn into_value(self) -> Value<$type> {
                Value::Constant(<$type>::from(self))
            }
        }

        impl<'a> From<&'a str> for $type {
            fn from(s: &'a str) -> Self {
                Self::new(s)
            }
        }

        impl IntoValue<$type> for &str {
            fn into_value(self) -> Value<$type> {
                Value::Constant(<$type>::from(self))
            }
        }

        impl IntoValue<$type> for Dynamic<String> {
            fn into_value(self) -> Value<$type> {
                Value::Dynamic(self.map_each_to())
            }
        }

        impl IntoValue<$type> for Dynamic<&'static str> {
            fn into_value(self) -> Value<$type> {
                Value::Dynamic(self.map_each(|s| <$type>::from(*s)))
            }
        }

        impl<'a> From<&'a String> for $type {
            fn from(s: &'a String) -> Self {
                Self::new(s.as_str())
            }
        }

        impl<T> PartialEq<T> for $type
        where
            T: PartialEq<str> + ?Sized,
        {
            fn eq(&self, other: &T) -> bool {
                other == self.as_str()
            }
        }

        impl InputStorage for $type {
            const MASKED: bool = $masked;

            fn as_str(&self) -> &str {
                &**self
            }

            fn as_string_mut(&mut self) -> &mut String {
                &mut *Arc::make_mut(&mut self.0)
            }
        }

        impl<T> InputValue<$type> for T where T: IntoDynamic<$type> {}
    };
}

impl_cow_string!(CowString, false);
impl_cow_string!(MaskedString, true);
