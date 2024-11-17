use crate::context::EventContext;
use crate::styles::components::{
    FontFamily, FontStyle, FontWeight, Heading1FontFamily, Heading1Style, Heading1Weight,
    Heading2FontFamily, Heading2Style, Heading2Weight, Heading3FontFamily, Heading3Style,
    Heading3Weight, Heading4FontFamily, Heading4Style, Heading4Weight, Heading5FontFamily,
    Heading5Style, Heading5Weight, Heading6FontFamily, Heading6Style, Heading6Weight, LineHeight,
    LineHeight1, LineHeight2, LineHeight3, LineHeight4, LineHeight5, LineHeight6, LineHeight7,
    LineHeight8, TextSize, TextSize1, TextSize2, TextSize3, TextSize4, TextSize5, TextSize6,
    TextSize7, TextSize8,
};
use crate::styles::{ComponentDefinition, IntoComponentValue, IntoDynamicComponentValue, Styles};
use crate::value::{Destination, IntoValue, Mutable, Value};
use crate::widget::{MakeWidget, WidgetRef, WrapperWidget};

/// A widget that applies a set of [`Styles`] to all contained widgets.
#[derive(Debug)]
pub struct Style {
    styles: Value<Styles>,
    child: WidgetRef,
}

impl Style {
    /// Returns a new widget that applies `styles` to `child` and any children
    /// it may have.
    pub fn new(styles: impl IntoValue<Styles>, child: impl MakeWidget) -> Self {
        Self {
            styles: styles.into_value(),
            child: WidgetRef::new(child),
        }
    }

    fn map_styles_mut<R>(&mut self, map: impl FnOnce(Mutable<'_, Styles>) -> R) -> R {
        match &mut self.styles {
            Value::Constant(styles) => map(Mutable::from(styles)),
            Value::Dynamic(dynamic) => dynamic.map_mut(map),
        }
    }

    /// Associates a style component with `self`.
    #[must_use]
    pub fn with<C: ComponentDefinition>(
        mut self,
        name: &C,
        component: impl IntoValue<C::ComponentType>,
    ) -> Style
    where
        Value<C::ComponentType>: IntoComponentValue,
    {
        self.map_styles_mut(|mut styles| {
            styles.insert(name, component.into_value());
        });
        self
    }

    /// Associates a style component with `self`, preventing the value from
    /// being inherited by child widgets.
    #[must_use]
    pub fn with_local<C: ComponentDefinition>(
        mut self,
        name: &C,
        component: impl IntoValue<C::ComponentType>,
    ) -> Style
    where
        Value<C::ComponentType>: IntoComponentValue,
    {
        self.map_styles_mut(|mut styles| {
            styles.insert_local_named(name.name().into_owned(), component.into_value());
        });
        self
    }

    /// Associates a style component with `self`, resolving its value using
    /// `dynamic` at runtime.
    #[must_use]
    pub fn with_dynamic<C: ComponentDefinition>(
        mut self,
        name: &C,
        dynamic: impl IntoDynamicComponentValue,
    ) -> Style
    where
        Value<C::ComponentType>: IntoComponentValue,
    {
        self.map_styles_mut(|mut styles| {
            styles.insert_dynamic(name, dynamic);
        });
        self
    }

    /// Associates a style component with `self`, resolving its value using
    /// `dynamic` at runtime. This value will not be inherited by child widgets.
    #[must_use]
    pub fn with_local_dynamic<C: ComponentDefinition>(
        mut self,
        name: &C,
        dynamic: impl IntoDynamicComponentValue,
    ) -> Style
    where
        Value<C::ComponentType>: IntoComponentValue,
    {
        self.map_styles_mut(|mut styles| {
            styles.insert_local_dynamic(name, dynamic);
        });
        self
    }

    /// Styles `self` with the largest of 6 heading styles.
    #[must_use]
    pub fn h1(self) -> Style {
        self.xxxx_large()
            .with_dynamic(&FontStyle, Heading1Style)
            .with_dynamic(&FontFamily, Heading1FontFamily)
            .with_dynamic(&FontWeight, Heading1Weight)
    }

    /// Styles `self` with the second largest of 6 heading styles.
    #[must_use]
    pub fn h2(self) -> Style {
        self.xxx_large()
            .with_dynamic(&FontStyle, Heading2Style)
            .with_dynamic(&FontFamily, Heading2FontFamily)
            .with_dynamic(&FontWeight, Heading2Weight)
    }

    /// Styles `self` with the third largest of 6 heading styles.
    #[must_use]
    pub fn h3(self) -> Style {
        self.xx_large()
            .with_dynamic(&FontStyle, Heading3Style)
            .with_dynamic(&FontFamily, Heading3FontFamily)
            .with_dynamic(&FontWeight, Heading3Weight)
    }

    /// Styles `self` with the third smallest of 6 heading styles.
    #[must_use]
    pub fn h4(self) -> Style {
        self.x_large()
            .with_dynamic(&FontStyle, Heading4Style)
            .with_dynamic(&FontFamily, Heading4FontFamily)
            .with_dynamic(&FontWeight, Heading4Weight)
    }

    /// Styles `self` with the second smallest of 6 heading styles.
    #[must_use]
    pub fn h5(self) -> Style {
        self.large()
            .with_dynamic(&FontStyle, Heading5Style)
            .with_dynamic(&FontFamily, Heading5FontFamily)
            .with_dynamic(&FontWeight, Heading5Weight)
    }

    /// Styles `self` with the smallest of 6 heading styles.
    #[must_use]
    pub fn h6(self) -> Style {
        self.default_size()
            .with_dynamic(&FontStyle, Heading6Style)
            .with_dynamic(&FontFamily, Heading6FontFamily)
            .with_dynamic(&FontWeight, Heading6Weight)
    }

    /// Styles `self` with the largest text size.
    #[must_use]
    pub fn xxxx_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize8)
            .with_dynamic(&LineHeight, LineHeight8)
    }

    /// Styles `self` with the second largest text size.
    #[must_use]
    pub fn xxx_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize7)
            .with_dynamic(&LineHeight, LineHeight7)
    }

    /// Styles `self` with the third largest text size.
    #[must_use]
    pub fn xx_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize6)
            .with_dynamic(&LineHeight, LineHeight6)
    }

    /// Styles `self` with the fourth largest text size.
    #[must_use]
    pub fn x_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize5)
            .with_dynamic(&LineHeight, LineHeight5)
    }

    /// Styles `self` with the fifth largest text size.
    #[must_use]
    pub fn large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize4)
            .with_dynamic(&LineHeight, LineHeight4)
    }

    /// Styles `self` with the third smallest text size.
    #[must_use]
    pub fn default_size(self) -> Style {
        self.with_dynamic(&TextSize, TextSize3)
            .with_dynamic(&LineHeight, LineHeight3)
    }

    /// Styles `self` with the second smallest text size.
    #[must_use]
    pub fn small(self) -> Style {
        self.with_dynamic(&TextSize, TextSize2)
            .with_dynamic(&LineHeight, LineHeight2)
    }

    /// Styles `self` with the smallest text size.
    #[must_use]
    pub fn x_small(self) -> Style {
        self.with_dynamic(&TextSize, TextSize1)
            .with_dynamic(&LineHeight, LineHeight1)
    }
}

impl WrapperWidget for Style {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        context.attach_styles(self.styles.clone());
    }
}
