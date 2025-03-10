//! Localization allows user interfaces to be presented in the user's native
//! locale (language and region).
//!
//! Localization in Cushy is powered by [Fluent](https://projectfluent.org/).
//! Fluent offers a variety of features that solve many common localization
//! problems. If you think your application might benefit from having multiple
//! languages, it might save a lot of time to build the application with
//! localization in mind.
//!
//! Thankfully, localization in Cushy is fairly straightforward. Wherever you
//! want to display a localizable message, use the [`Localize`] type or the
//! [`localize!`](crate::localize) macro:
//!
//! ```rust
//! use cushy::localization::Localize;
//! use cushy::localize;
//!
//! let message = Localize::new("hello-world");
//! let message = localize!("hello-world");
//! ```
//!
//! Regardless of which style you prefer, `message` now contains a localizable
//! message. When the application is running, wherever `message` is used, the
//! message will be looked up in the current locale.
//!
//! Localization messages are resolved through the application's
//! [`Cushy`](crate::Cushy) instance.
//! [`Cushy::localizations()`](crate::Cushy::localizations) returns the global
//! [`Localizations`] collection, which [`Localization`]s can be added to.
//! Consider this simple example:
//!
//! ```rust
//! use cushy::localization::Localization;
//! use cushy::{localize, Open, PendingApp};
//!
//! # fn main() {
//! #[cushy::main]
//! fn main(app: &mut PendingApp) -> cushy::Result {
//!     app.cushy()
//!         .localizations()
//!         .add_default(Localization::for_language("en-US", "hello = Hello World!").unwrap());
//!     app.cushy()
//!         .localizations()
//!         .add_default(Localization::for_language("es-MX", "hello = ¡Hola Mundo!").unwrap());
//!     app.cushy()
//!         .localizations()
//!         .add_default(Localization::for_language("fr-FR", "hello = Bonjour monde!").unwrap());
//!
//!     localize!("hello").open(app)?;
//!
//!     Ok(())
//! }
//! # }
//! ```
//!
//! Additionally, Fluent supports providing arguments to localization messages:
//!
//!
//! ```rust
//! use cushy::localization::Localization;
//! use cushy::{localize, Open, PendingApp};
//!
//! # fn main() {
//! #[cushy::main]
//! fn main(app: &mut PendingApp) -> cushy::Result {
//!     app.cushy()
//!         .localizations()
//!         .add_default(Localization::for_language("en-US", "hello-user = Hello {$name}!").unwrap());
//!     app.cushy()
//!         .localizations()
//!         .add_default(Localization::for_language("es-MX", "hello-user = ¡Hola {$name}!").unwrap());
//!     app.cushy()
//!         .localizations()
//!         .add_default(Localization::for_language("fr-FR", "hello-user = Bonjour {$name}!").unwrap());
//!
//!     localize!("hello", "user" => "Ecton").open(app)?;
//!
//!     Ok(())
//! }
//! # }
//! ```
//!
//! # Locale Fallback Behavior
//!
//! Cushy attempts to find an exact match between the current locale and a
//! loaded [`Localization`]. If an exact match is not found, [`fluent_langneg`]
//! is used to try to find a fallback locale. If the message being localized
//! cannot be found in either of these locales, the message is looked up in the
//! *default locale*.
//!
//! Cushy has the concept of a *default locale*. There is no default locale
//! until either [`Localizations::add_default`] or
//! [`Localizations::set_default_locale`] are executed. Once a default locale is
//! established, any messages that cannot be found in the current locale or a
//! fallback locale will be localized using the default locale as a final
//! effort.
//!
//! Using the default locale can be convenient, but it can also make it harder
//! to visually notice when a message is missing from a particular locale. When
//! relying on third parties to provide localizations, it can be beneficial to
//! ensure that a valid message is always shown even if a localized message has
//! not been provided yet.

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
use crate::reactive::value::{Dynamic, DynamicRead, Generation, IntoValue, Source, Value};
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

/// A context that is used while localizing values.
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
        self.localizations()
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
        self.widget.localizations()
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
        self.widget.localizations()
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
        self.widget.localizations()
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
#[derive(Clone, Debug, PartialEq)]
pub struct Localize {
    key: Cow<'static, str>,
    args: Vec<(String, Value<FluentValue<'static>>)>,
}

impl IntoValue<MaybeLocalized> for Localize {
    fn into_value(self) -> Value<MaybeLocalized> {
        Value::Constant(MaybeLocalized::from(self))
    }
}

impl Localize {
    /// Returns a value that localizes `key` at runtime.
    ///
    /// The `key` should refer to a valid message identifier in the loaded
    /// [`Localizations`] for the application.
    pub fn new(key: impl Into<Cow<'static, str>>) -> Self {
        Self {
            key: key.into(),
            args: Vec::new(),
        }
    }

    /// Returns localized value using `context`.
    pub fn localize(&self, context: &impl LocalizationContext) -> String {
        let mut localized = String::new();
        self.localize_into(context, &mut localized)
            .assert("format success");
        localized
    }

    /// Add a named argument, which can be used with parameterized messages.
    ///
    /// See [Variables](https://projectfluent.org/fluent/guide/variables.html)
    #[must_use]
    pub fn arg(
        mut self,
        key: impl Into<String>,
        value: impl IntoValue<FluentValue<'static>>,
    ) -> Self {
        self.args.push((key.into(), value.into_value()));
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
        let mut state = context.localizations().state.lock();
        // When localizing, we need mut access to update the FallbackLocales
        // cache. We don't want fallback locale renegotation to cause extra
        // invalidations.
        state.prevent_notifications();

        let Some((bundle, value)) = state
            .localize(self, &locale)
            .and_then(|(bundle, message)| message.value().map(|value| (bundle, value)))
        else {
            tracing::warn!("missing localization of `{}` for {locale}", self.key);
            return f.write_str(&format!("$missing {} for {locale}$", self.key));
        };

        let mut err = vec![];
        let args = self.get_args(context);
        bundle.write_pattern(f, value, Some(&args), &mut err)?;

        for err in err {
            tracing::error!("error localizing {} in {locale}: {err}", self.key);
        }

        Ok(())
    }
}

/// Returns a message localized in the current locale.
///
/// The first argument to this macro is the unique id/key of the message being
/// localized. After the initial argument, the remaning arguments are expected
/// to be `name => value` pairs.
///
/// ```rust
/// use cushy::localize;
///
/// let message = localize!("welcome-message");
///
/// let message = localize!("welcome-message", "user" => "Ecton");
/// ```
///
/// This macro always returns a [`Localize`].
#[macro_export]
macro_rules! localize {
    ($key:expr) => {
        $crate::localization::Localize::new($key)
    };
    ($key:expr, $($name:expr => $arg:expr),*) => {
        {
            let mut localize = $crate::localization::Localize::new($key);
            $(
                localize = localize.arg($name, $arg);
            )*
            localize
        }
    };
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
    /// Add a localization to this collection.
    ///
    /// Any errors will be output using `tracing`.
    pub fn add(&self, localization: Localization) {
        let mut state = self.state.lock();

        state.add(localization);
    }

    /// Add a localization to this collection, setting this localizations's
    /// locale as the default locale.
    ///
    /// Any errors will be output using `tracing`.
    ///
    /// See [`Localizations::set_default_locale`] for more information about
    /// what the default locale is for.
    pub fn add_default(&self, localization: Localization) {
        let mut state = self.state.lock();
        state.default_locale = localization.locale.clone();

        state.add(localization);
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
    ///
    /// See [`Localizations::set_default_locale`] for more information about
    /// what the default locale is for.
    #[must_use]
    pub fn default_locale(&self) -> LanguageIdentifier {
        self.state.read().default_locale.clone()
    }

    /// Returns a dynamic that controls the expected locale of the user for the
    /// application.
    ///
    /// This dynamic contains [`Locale::System`] by default.
    ///
    /// Changing the value contained by this dynamaic will update the locale for
    /// the entire application. The [`Localized`](crate::widgets::Localized)
    /// widget can be used to localize a section of an user interface.
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

#[derive(Default)]
struct TranslationState {
    fallback_locales: FallbackLocales,
    default_locale: LanguageIdentifier,
    all_locales: Vec<LanguageIdentifier>,
    loaded_bundles: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>,
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
            .loaded_bundles
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
        self.loaded_bundles
            .get(locale)
            .and_then(|bundle| {
                bundle
                    .get_message(&message.key)
                    .map(|message| (bundle, message))
            })
            .or_else(|| {
                self.fallback_locales
                    .fallback_for(locale, &self.all_locales, &self.default_locale)
                    .and_then(|fallback| self.loaded_bundles.get(fallback))
                    .and_then(|bundle| {
                        bundle
                            .get_message(&message.key)
                            .map(|message| (bundle, message))
                    })
            })
            .or_else(|| {
                self.loaded_bundles
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
