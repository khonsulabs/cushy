//! Localization support for Gooey using [Project Fluent](https://projectfluent.org).

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(clippy::if_not_else, clippy::module_name_repetitions)]
#![cfg_attr(doc, warn(rustdoc::all))]

use std::{borrow::Cow, collections::HashMap, fmt::Debug};

pub use fluent;
use fluent::{
    bundle::FluentBundle,
    types::{FluentNumber, FluentNumberOptions},
    FluentArgs, FluentResource, FluentValue,
};
use fluent_langneg::NegotiationStrategy;
use gooey_core::{
    unic_langid::LanguageIdentifier, LocalizationParameter, LocalizationParameters, Localizer,
};
use intl_memoizer::concurrent::IntlLangMemoizer;

/// A [`Localizer`] for Gooey that utilizes [`fluent`].
///
/// During construction, [`FluentLocalizer`] gathers a unique list of primary
/// [`LanguageIdentifier`]s for each of the [`FluentBundle`]s provided. When
/// localizing, this list is filtered using
/// [`fluent-langneg`](https://crates.io/crates/fluent-langneg), and resources
/// are searched through all matching bundles in the order determined by
/// `fluent-langneg`.
///
/// This type supports inserting more than one bundle for a given
/// [`LanguageIdentifier`].
#[must_use]
pub struct FluentLocalizer {
    default_language: LanguageIdentifier,
    languages: Vec<LanguageIdentifier>,
    bundles: HashMap<LanguageIdentifier, Vec<FluentBundle<FluentResource, IntlLangMemoizer>>>,
}

impl FluentLocalizer {
    /// Creates a new instance that localizes using `bundles`.
    pub fn new(
        default_language: LanguageIdentifier,
        bundles: Vec<FluentBundle<FluentResource, IntlLangMemoizer>>,
    ) -> Self {
        let mut bundle_map = HashMap::<
            LanguageIdentifier,
            Vec<FluentBundle<FluentResource, IntlLangMemoizer>>,
        >::new();
        let mut languages = Vec::new();
        for mut bundle in bundles {
            // TODO BiDi support. For now, this causes rendering issues.
            bundle.set_use_isolating(false);
            let primary_locale = bundle
                .locales
                .first()
                .expect("All bundles need at least one locale");
            if let Some(existing) = bundle_map.get_mut(primary_locale) {
                existing.push(bundle);
            } else {
                languages.push(primary_locale.clone());
                bundle_map.insert(primary_locale.clone(), vec![bundle]);
            }
        }
        Self {
            default_language,
            bundles: bundle_map,
            languages,
        }
    }
}

impl Debug for FluentLocalizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FluentLocalizer").finish_non_exhaustive()
    }
}

impl Localizer for FluentLocalizer {
    fn localize<'a>(
        &self,
        key: &str,
        parameters: Option<LocalizationParameters<'a>>,
        language: &LanguageIdentifier,
    ) -> String {
        for language in fluent_langneg::negotiate_languages(
            &[language],
            &self.languages,
            Some(&self.default_language),
            NegotiationStrategy::Filtering,
        ) {
            if let Some(language_bundles) = self.bundles.get(language) {
                for bundle in language_bundles {
                    if let Some(message) = bundle.get_message(key) {
                        let mut errors = Vec::new();
                        if let Some(localized) = message.value().map(|pattern| {
                            let args = parameters.as_ref().map(|parameters| {
                                let mut args = FluentArgs::new();
                                for (key, value) in parameters.iter() {
                                    args.set(
                                        key,
                                        match value {
                                            LocalizationParameter::String(value) => {
                                                FluentValue::String(Cow::Borrowed(value))
                                            }
                                            LocalizationParameter::Numeric(value) => {
                                                FluentValue::Number(FluentNumber::new(
                                                    *value,
                                                    FluentNumberOptions::default(),
                                                ))
                                            }
                                        },
                                    );
                                }
                                args
                            });
                            bundle
                                .format_pattern(pattern, args.as_ref(), &mut errors)
                                .to_string()
                        }) {
                            return localized;
                        }
                        log::error!(
                            "Error localizing '{}' in language {}: {:?}",
                            key,
                            language,
                            errors
                        );
                    }
                }
            }
        }

        String::from(key)
    }
}
