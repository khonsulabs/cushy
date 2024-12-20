//! A read-only text widget.

use std::borrow::Cow;
use std::fmt::{Debug, Display, Write};

use figures::units::{Px, UPx};
use figures::{IntoUnsigned, Point, Round, Size, Zero};
use kludgine::text::{MeasuredText, Text, TextOrigin};
use kludgine::{cosmic_text, CanRenderTo, Color, DrawableExt};

use super::input::CowString;
use crate::context::{FontSettings, GraphicsContext, LayoutContext, Trackable, WidgetContext};
use crate::styles::components::{HorizontalAlignment, TextColor, VerticalAlignment};
use crate::styles::{HorizontalAlign, VerticalAlign};
use crate::value::{
    Dynamic, DynamicReader, Generation, IntoDynamic, IntoReadOnly, IntoValue, ReadOnly, Value,
};
use crate::widget::{MakeWidgetWithTag, Widget, WidgetInstance, WidgetTag};
use crate::window::WindowLocal;
use crate::{ConstraintLimit, FitMeasuredSize};

/// A read-only text widget.
#[derive(Debug)]
pub struct Label<T> {
    /// The contents of the label.
    pub display: ReadOnly<T>,
    /// The behavior to use when too much text is able to be displayed on a
    /// single line.
    pub overflow: Value<LabelOverflow>,
    displayed: String,
    prepared_text: WindowLocal<LabelCache>,
}

impl<T> Label<T>
where
    T: Debug + DynamicDisplay + Send + 'static,
{
    /// Returns a new label that displays `text`, wrapping if necessary to fit
    /// the content in the provided space.
    pub fn new(text: impl IntoReadOnly<T>) -> Self {
        Self {
            display: text.into_read_only(),
            overflow: Value::Constant(LabelOverflow::WordWrap),
            displayed: String::new(),
            prepared_text: WindowLocal::default(),
        }
    }

    /// Sets the behavior when more text than can fit on a single line is
    /// displayed.
    #[must_use]
    pub fn overflow(mut self, overflow: impl IntoValue<LabelOverflow>) -> Self {
        self.overflow = overflow.into_value();
        self
    }

    fn prepared_text(
        &mut self,
        context: &mut GraphicsContext<'_, '_, '_, '_>,
        color: Color,
        mut width: Px,
        align: HorizontalAlign,
    ) -> &MeasuredText<Px> {
        let align = match align {
            HorizontalAlign::Left => cosmic_text::Align::Left,
            HorizontalAlign::Center => cosmic_text::Align::Center,
            HorizontalAlign::Right => cosmic_text::Align::Right,
        };
        let overflow = self.overflow.get_tracking_invalidate(context);
        if overflow == LabelOverflow::Clip {
            width = Px::MAX;
        }
        context.apply_current_font_settings();

        let mut cache_key = LabelCacheKey {
            generation: self.display.generation(),
            display_generation: self.display.map(|display| display.generation(context)),
            width,
            color,
            settings: context.current_font_settings(),
            align,
        };

        match self.prepared_text.get(context) {
            Some(cache)
                if cache.text.can_render_to(&context.gfx) && cache_key.is_valid_for(cache) => {}
            _ => {
                let (measured, display_generation) = self.display.map(|text| {
                    self.displayed.clear();
                    if let Err(err) = write!(&mut self.displayed, "{}", text.as_display(context)) {
                        tracing::error!("Error invoking Display: {err}");
                    }
                    (
                        context
                            .gfx
                            .measure_text(Text::new(&self.displayed, color).align(align, width)),
                        text.generation(context),
                    )
                });
                cache_key.display_generation = display_generation;
                self.prepared_text.set(
                    context,
                    LabelCache {
                        text: measured,
                        key: cache_key,
                    },
                );
            }
        }

        self.prepared_text
            .get(context)
            .map(|cache| &cache.text)
            .expect("always initialized")
    }
}

impl<T> Widget for Label<T>
where
    T: Debug + DynamicDisplay + Send + 'static,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        self.display.invalidate_when_changed(context);

        let align = context.get(&HorizontalAlignment);
        let valign = context.get(&VerticalAlignment);

        let text_color = context.get(&TextColor);

        let prepared_text =
            self.prepared_text(context, text_color, context.gfx.region().size.width, align);

        let y_offset = match valign {
            VerticalAlign::Top => Px::ZERO,
            VerticalAlign::Center => {
                (context.gfx.region().size.height - prepared_text.size.height) / 2
            }
            VerticalAlign::Bottom => context.gfx.region().size.height - prepared_text.size.height,
        };

        context.gfx.draw_measured_text(
            prepared_text.translate_by(Point::new(Px::ZERO, y_offset)),
            TextOrigin::TopLeft,
        );
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let align = context.get(&HorizontalAlignment);
        let color = context.get(&TextColor);
        let width = available_space.width.max().try_into().unwrap_or(Px::MAX);
        let prepared = self.prepared_text(context, color, width, align);

        available_space.fit_measured(prepared.size.into_unsigned().ceil())
    }

    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_tuple("Label").field(&self.display).finish()
    }

    fn unmounted(&mut self, context: &mut crate::context::EventContext<'_>) {
        self.prepared_text.clear_for(context);
    }
}

macro_rules! impl_make_widget {
    ($($type:ty => $kind:ty),*) => {
        $(impl MakeWidgetWithTag for $type {
            fn make_with_tag(self, id: WidgetTag) -> WidgetInstance {
                Label::<$kind>::new(self).make_with_tag(id)
            }
        })*
    };
}

impl_make_widget!(
    &'_ str => String,
    String => String,
    CowString => CowString,
    Dynamic<String> => String,
    Dynamic<&'static str> => &'static str,
    Value<String> => String,
    ReadOnly<String> => String
);

impl MakeWidgetWithTag for Cow<'_, str> {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        Label::new(self.into_owned()).make_with_tag(tag)
    }
}

impl MakeWidgetWithTag for &'_ String {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        Label::new(self.clone()).make_with_tag(tag)
    }
}

/// The overflow behavior for a [`Label`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum LabelOverflow {
    /// Any text that cannot be drawn on a single line will be clipped to the
    /// bounds of the label.
    Clip,
    /// Wraps text at the boundaries between words and whitespace while
    /// attaching punctuation to the non-wrapped word when possible.
    WordWrap,
}

#[derive(Debug)]
struct LabelCache {
    text: MeasuredText<Px>,
    key: LabelCacheKey,
}

#[derive(Debug)]
struct LabelCacheKey {
    generation: Option<Generation>,
    display_generation: Option<Generation>,
    width: Px,
    color: Color,
    settings: FontSettings,
    align: cosmic_text::Align,
}

impl LabelCacheKey {
    pub fn is_valid_for(&self, cache: &LabelCache) -> bool {
        if self.generation == cache.key.generation
            && self.display_generation == cache.key.display_generation
            && self.color == cache.key.color
            && self.settings == cache.key.settings
            && self.align == cache.key.align
        {
            if self.align == cosmic_text::Align::Left {
                self.width <= cache.key.width && cache.text.size.width <= self.width
            } else {
                self.width == cache.key.width
            }
        } else {
            false
        }
    }
}

/// A context-aware [`Display`] implementation.
///
/// This trait is automatically implemented for all types that implement
/// [`Display`].
pub trait DynamicDisplay {
    /// Returns a generation representing the current state of the dynamic.
    ///
    /// To ensure the contents are recached by a [`Label`] widget, return a
    /// unique value from this function each time the contents are updated.
    #[allow(unused_variables)]
    fn generation(&self, context: &WidgetContext<'_>) -> Option<Generation> {
        None
    }

    /// Format `self` with any needed information from `context`.
    fn fmt(&self, context: &WidgetContext<'_>, f: &mut std::fmt::Formatter<'_>)
        -> std::fmt::Result;

    /// Returns a type that implements [`Display`].
    fn as_display<'display, 'ctx>(
        &'display self,
        context: &'display WidgetContext<'ctx>,
    ) -> DynamicDisplayer<'display, 'ctx>
    where
        Self: Sized,
    {
        DynamicDisplayer(self, context)
    }
}

impl<T> DynamicDisplay for T
where
    T: Display,
{
    fn fmt(
        &self,
        _context: &WidgetContext<'_>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        self.fmt(f)
    }
}

/// A generic [`Display`] implementation for a [`DynamicDisplay`] implementor.
pub struct DynamicDisplayer<'a, 'w>(&'a dyn DynamicDisplay, &'a WidgetContext<'w>);

impl Display for DynamicDisplayer<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(self.1, f)
    }
}

/// A type that can be displayed as a [`Label`].
pub trait Displayable<T>
where
    T: Debug + Display + Send + 'static,
{
    /// Returns this value as a displayable reader.
    fn into_displayable(self) -> DynamicReader<T>;

    /// Returns `self` being `Display`ed in a [`Label`] widget.
    fn into_label(self) -> Label<T>
    where
        Self: Sized,
        T: Debug + Display + Send + 'static,
    {
        Label::new(self.into_displayable())
    }

    /// Returns `self` being `Display`ed in a [`Label`] widget.
    fn to_label(&self) -> Label<T>
    where
        Self: Clone,
    {
        self.clone().into_label()
    }
}

impl<T> Displayable<T> for T
where
    T: Debug + Display + Send + 'static,
{
    fn into_displayable(self) -> DynamicReader<T> {
        Dynamic::new(self).into_reader()
    }
}

impl<T> Displayable<T> for Dynamic<T>
where
    T: Debug + Display + Send + 'static,
{
    fn into_displayable(self) -> DynamicReader<T> {
        self.into_reader()
    }
}

impl<T> Displayable<T> for DynamicReader<T>
where
    T: Debug + Display + Send + 'static,
{
    fn into_displayable(self) -> DynamicReader<T> {
        self
    }
}

impl<T> Displayable<T> for Value<T>
where
    T: Debug + Display + Send + 'static,
{
    fn into_displayable(self) -> DynamicReader<T> {
        self.into_dynamic().into_reader()
    }
}
