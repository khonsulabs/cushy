//! Localization allows UIs to be created that support more than just one language.
//!
//! The basic idea is that instead of using strings throughout the application, you use keys that
//! refer to messages in translation files.
//!
//! The Fluent crate is used as a backend.  Translations are stored in `.ftl` files.
//!
//! Basic example of creating a label:
//! ```rust
//! use cushy::localization::Localize;
//! let label = Localize::new("message-hello-world").into_label();
//! ```
//!
//! Translation files are added to the `Cushy` app instance, see [`Cushy::translations()`](crate::app::Cushy::translations).

use core::fmt;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentMessage, FluentResource, FluentValue};
use intentional::Assert;
use kempt::{map, Map};
use unic_langid::{LanguageIdentifier, LanguageIdentifierError};

use crate::context::{EventContext, GraphicsContext, LayoutContext, Trackable, WidgetContext};
use crate::value::{Dynamic, DynamicRead, Generation, IntoValue, Source, Value};
use crate::widget::{MakeWidgetWithTag, WidgetInstance, WidgetTag};
use crate::widgets::label::DynamicDisplay;
use crate::MaybeLocalized;

impl MaybeLocalized {
    /// Returns the localized version of this string, using `context` to
    /// localize.
    pub fn localize<'a>(&'a self, context: &impl LocalizationContext) -> Cow<'a, str> {
        match self {
            MaybeLocalized::Text(value) => Cow::Borrowed(value),
            MaybeLocalized::Localized(value) => Cow::Owned(value.localize(context)),
        }
    }
}

pub(crate) struct WindowTranslationContext<'a>(pub(crate) &'a Localizations);

/// A context that can used while localizing values.
pub trait LocalizationContext {
    /// Returns the current locale of this context.
    fn locale(&self) -> LanguageIdentifier;
    /// Returns the localizations for this context.
    fn localizations(&self) -> &Localizations;
    /// Invalidates `trackable` when changed.
    ///
    /// Some values are localized outside of the context of a window being
    /// opened: for example, the Window's title. In situations like these,
    /// invalidation is ignored.
    fn invalidate_when_changed(&self, trackable: &impl Trackable);
}

impl LocalizationContext for WidgetContext<'_> {
    fn locale(&self) -> LanguageIdentifier {
        self.locale().get_tracking_invalidate(self)
    }

    fn localizations(&self) -> &Localizations {
        self.translations()
    }

    fn invalidate_when_changed(&self, trackable: &impl Trackable) {
        trackable.invalidate_when_changed(self);
    }
}

impl LocalizationContext for EventContext<'_> {
    fn locale(&self) -> LanguageIdentifier {
        self.widget.locale().get_tracking_invalidate(&self.widget)
    }

    fn localizations(&self) -> &Localizations {
        self.widget.translations()
    }

    fn invalidate_when_changed(&self, trackable: &impl Trackable) {
        trackable.invalidate_when_changed(&self.widget);
    }
}

impl LocalizationContext for GraphicsContext<'_, '_, '_, '_> {
    fn locale(&self) -> LanguageIdentifier {
        self.widget.locale().get_tracking_invalidate(&self.widget)
    }

    fn localizations(&self) -> &Localizations {
        self.widget.translations()
    }

    fn invalidate_when_changed(&self, trackable: &impl Trackable) {
        trackable.invalidate_when_changed(&self.widget);
    }
}

impl LocalizationContext for LayoutContext<'_, '_, '_, '_> {
    fn locale(&self) -> LanguageIdentifier {
        self.widget.locale().get_tracking_invalidate(&self.widget)
    }

    fn localizations(&self) -> &Localizations {
        self.widget.translations()
    }

    fn invalidate_when_changed(&self, trackable: &impl Trackable) {
        trackable.invalidate_when_changed(&self.widget);
    }
}

impl LocalizationContext for WindowTranslationContext<'_> {
    fn locale(&self) -> LanguageIdentifier {
        LanguageIdentifier::default()
    }

    fn localizations(&self) -> &Localizations {
        self.0
    }

    fn invalidate_when_changed(&self, _trackable: &impl Trackable) {}
}

impl From<Localize> for MaybeLocalized {
    fn from(value: Localize) -> Self {
        Self::Localized(value)
    }
}

impl DynamicDisplay for MaybeLocalized {
    fn fmt(
        &self,
        context: &WidgetContext<'_>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            MaybeLocalized::Text(text) => Display::fmt(text, f),
            MaybeLocalized::Localized(localize) => DynamicDisplay::fmt(localize, context, f),
        }
    }
}

/// The primary of defining localized message
#[derive(Clone, Debug)]
pub struct Localize {
    key: String,
    args: HashMap<String, Value<FluentValue<'static>>>,
}

impl IntoValue<MaybeLocalized> for Localize {
    fn into_value(self) -> Value<MaybeLocalized> {
        Value::Constant(MaybeLocalized::from(self))
    }
}

impl Localize {
    /// Create a new [`Localization`] instance.
    ///
    /// The `key` should refer to a valid message identifier in the localization files.
    /// See [Writing Text](https://projectfluent.org/fluent/guide/text.html)
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: HashMap::new(),
        }
    }

    /// Localizes this message using the given translation context.
    pub fn localize(&self, context: &impl LocalizationContext) -> String {
        let mut localized = String::new();
        self.localize_into(context, &mut localized)
            .assert("format success");
        localized
    }

    /// Add an argument which can be used by the `.ftl` files.
    ///
    /// See [Variables](https://projectfluent.org/fluent/guide/variables.html)
    #[must_use]
    pub fn arg(
        mut self,
        key: impl Into<String>,
        value: impl IntoValue<FluentValue<'static>>,
    ) -> Self {
        self.args.insert(key.into(), value.into_value());
        self
    }

    fn get_args(&self, context: &impl LocalizationContext) -> FluentArgs {
        let mut res = FluentArgs::new();
        for (name, arg) in &self.args {
            context.invalidate_when_changed(arg);
            res.set(name.to_owned(), arg.get());
        }
        res
    }

    fn localize_into<W: fmt::Write>(
        &self,
        context: &impl LocalizationContext,
        f: &mut W,
    ) -> fmt::Result {
        let locale = context.locale();

        let translations = context.localizations();
        let mut state = translations.state.lock();
        // When localizing, we need mut access to update the FallbackLocales
        // cache. We don't want fallback locale renegotation to cause extra
        // invalidations.
        state.prevent_notifications();

        let Some((bundle, message)) = state.localize(self, &locale) else {
            return f.write_str(&format!("No message. locale: {locale}, key: {}", self.key));
        };

        let Some(value) = message.value() else {
            return f.write_str(&format!("No value. locale: {locale}, key: {}", self.key));
        };

        let mut err = vec![];
        let args = self.get_args(context);
        let res = bundle.format_pattern(value, Some(&args), &mut err);

        if err.is_empty() {
            f.write_str(&res)
        } else {
            f.write_str(&format!(
                "{} {{Error. locale: {}, key: {}, cause: {:?}}}",
                locale, self.key, res, err
            ))
        }
    }
}

macro_rules! impl_into_fluent_value {
    ($($ty:ty)+) => {
        $(impl_into_fluent_value!(. $ty);)+
    };
    (. $ty:ty) => {
        impl IntoValue<FluentValue<'static>> for $ty {
            fn into_value(self) -> Value<FluentValue<'static>> {
                Value::Constant(FluentValue::from(self))
            }
        }
        impl IntoValue<FluentValue<'static>> for Dynamic<$ty> {
            fn into_value(self) -> Value<FluentValue<'static>> {
                (&self).into_value()
            }
        }
        impl IntoValue<FluentValue<'static>> for &Dynamic<$ty> {
            fn into_value(self) -> Value<FluentValue<'static>> {
                Value::Dynamic(self.map_each_into())
            }
        }
    };
}

impl_into_fluent_value!(i8 i16 i32 i64 i128 isize);
impl_into_fluent_value!(u8 u16 u32 u64 u128 usize);
impl_into_fluent_value!(f32 f64);
impl_into_fluent_value!(String &'static str);

impl DynamicDisplay for Localize {
    fn generation(&self, context: &WidgetContext<'_>) -> Option<Generation> {
        let mut generation = context.localizations().state.generation();
        if let Some(locale_generation) = context.locale().generation() {
            generation += locale_generation;
        }

        Some(
            self.args
                .iter()
                .filter_map(|(_name, value)| value.generation())
                .fold(generation, |generation, value_generation| {
                    generation + value_generation
                }),
        )
    }

    fn fmt(&self, context: &WidgetContext<'_>, f: &mut Formatter<'_>) -> fmt::Result {
        self.localize_into(context, f)
    }
}

impl MakeWidgetWithTag for Localize {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        self.into_label().make_with_tag(tag)
    }
}

/// A localization for a specific locale.
pub struct Localization {
    /// The locale this localization applies to.
    pub locale: LanguageIdentifier,
    /// The Fluent (.ftl) source for this localization.
    pub fluent: String,
}

impl Localization {
    /// Returns a new localization from the given language and Fluent source.
    pub fn new(language: LanguageIdentifier, fluent: impl Into<String>) -> Self {
        Self {
            locale: language,
            fluent: fluent.into(),
        }
    }

    /// Returns a new localization from the given language and Fluent source.
    ///
    /// # Errors
    ///
    /// Returns an error if `language` is not a valid Unicode language
    /// identifier.
    pub fn for_language(
        language: &str,
        fluent: impl Into<String>,
    ) -> Result<Self, LanguageIdentifierError> {
        Ok(Self {
            locale: LanguageIdentifier::from_str(language)?,
            fluent: fluent.into(),
        })
    }
}

/// A locale (language and region)
#[derive(Default, Debug, Clone, PartialEq)]
pub enum Locale {
    /// Detect the locale of the system running the application.
    #[default]
    System,
    /// Use a specific locale with the given id.
    WithId(LanguageIdentifier),
}

/// A collection of localizations to apply to a Cushy application.
#[derive(Clone, Default)]
pub struct Localizations {
    state: Dynamic<TranslationState>,
    locale: Dynamic<Locale>,
}

impl Localizations {
    /// Add a `Fluent` translation file for a given locale.
    ///
    /// Note the `.ftl` file is not immediately parsed.
    pub fn add(&self, translation: Localization) {
        let mut state = self.state.lock();

        state.add(translation);
    }

    /// Add a `Fluent` translation file for a given locale, setting this
    /// translation's locale as the default locale for this application.
    ///
    /// Note the `.ftl` file is not immediately parsed.
    pub fn add_default(&self, translation: Localization) {
        let mut state = self.state.lock();
        state.default_locale = translation.locale.clone();

        state.add(translation);
    }

    /// Sets the locale to use as a fallback when the currently set or detected
    /// locales cannot localize a given value.
    ///
    /// This allows incompatible languages to be used as a "final" fallback. If
    /// the application is originally developed in the United States in English,
    /// for example, the default locale could be set to `en-US` and any missing
    /// strings from other languages will still be shown using the `en-US`
    /// values.
    pub fn set_default_locale(&self, locale: LanguageIdentifier) {
        self.state.lock().default_locale = locale;
    }

    /// Returns the default locale.
    #[must_use]
    pub fn default_locale(&self) -> LanguageIdentifier {
        self.state.read().default_locale.clone()
    }

    /// Returns a dynamic that controls the expected locale of the user for the
    /// application.
    ///
    /// This dynamic contains [`Locale::System`] by default.
    #[must_use]
    pub const fn user_locale(&self) -> &Dynamic<Locale> {
        &self.locale
    }

    #[must_use]
    pub(crate) fn effective_locale(&self, context: &WidgetContext<'_>) -> LanguageIdentifier {
        match self.user_locale().get_tracking_invalidate(context) {
            Locale::System => sys_locale::get_locale()
                .and_then(|locale| LanguageIdentifier::from_str(&locale).ok())
                .unwrap_or_else(|| self.default_locale()),
            Locale::WithId(id) => id,
        }
    }
}

struct TranslationState {
    fallback_locales: FallbackLocales,
    default_locale: LanguageIdentifier,
    all_locales: Vec<LanguageIdentifier>,
    loaded_translations: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
}

impl Default for TranslationState {
    fn default() -> Self {
        Self {
            fallback_locales: FallbackLocales::default(),
            default_locale: LanguageIdentifier::default(),
            all_locales: Vec::new(),
            loaded_translations: HashMap::from([(
                LanguageIdentifier::default(),
                FluentBundle::new_concurrent(vec![LanguageIdentifier::default()]),
            )]),
        }
    }
}

impl TranslationState {
    fn add(&mut self, translation: Localization) {
        let res = match FluentResource::try_new(translation.fluent) {
            Ok(res) => res,
            Err((res, errors)) => {
                for err in errors {
                    tracing::error!("error parsing {} localization: {err}", translation.locale);
                }
                res
            }
        };
        let bundle = self
            .loaded_translations
            .entry(translation.locale.clone())
            .or_insert_with(|| FluentBundle::new_concurrent(vec![translation.locale.clone()]));
        if let Err(errors) = bundle.add_resource(res) {
            for err in errors {
                tracing::error!("error adding {} localization: {err}", translation.locale);
            }
        }
        self.fallback_locales.clear();
    }

    #[must_use]
    fn localize<'a>(
        &'a mut self,
        message: &Localize,
        locale: &LanguageIdentifier,
    ) -> Option<(&'a FluentBundle<FluentResource>, FluentMessage<'a>)> {
        self.loaded_translations
            .get(locale)
            .and_then(|bundle| {
                bundle
                    .get_message(&message.key)
                    .map(|message| (bundle, message))
            })
            .or_else(|| {
                self.fallback_locales
                    .fallback_for(locale, &self.all_locales, &self.default_locale)
                    .and_then(|fallback| self.loaded_translations.get(fallback))
                    .and_then(|bundle| {
                        bundle
                            .get_message(&message.key)
                            .map(|message| (bundle, message))
                    })
            })
            .or_else(|| {
                self.loaded_translations
                    .get(&self.default_locale)
                    .and_then(|bundle| {
                        bundle
                            .get_message(&message.key)
                            .map(|message| (bundle, message))
                    })
            })
    }
}

#[derive(Default)]
struct FallbackLocales(Map<LanguageIdentifier, LanguageIdentifier>);

impl FallbackLocales {
    fn clear(&mut self) {
        self.0.clear();
    }

    fn fallback_for<'a>(
        &'a mut self,
        language: &LanguageIdentifier,
        available_locales: &[LanguageIdentifier],
        default_locale: &LanguageIdentifier,
    ) -> Option<&'a LanguageIdentifier> {
        match self.0.entry(language) {
            map::Entry::Occupied(entry) => Some(entry.into_mut()),
            map::Entry::Vacant(vacant) => {
                let fallback = fluent_langneg::negotiate::negotiate_languages(
                    &[language.clone()],
                    available_locales,
                    Some(default_locale),
                    fluent_langneg::NegotiationStrategy::Filtering,
                )
                .into_iter()
                .next()?;

                Some(vacant.insert(fallback.clone()))
            }
        }
    }
}
