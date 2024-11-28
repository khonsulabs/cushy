use core::fmt;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;
use cushy::widgets::Label;
use crate::context::WidgetContext;
use crate::value::{Dynamic, Generation, IntoValue, Value};
use crate::widgets::label::{DynamicDisplay};

pub struct Localize<'args> {
    key: String,
    args: HashMap<String, Value<FluentValue<'args>>>
}

impl<'args> Debug for Localize<'args> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Point")
            .field("key", &self.key)
            .field("args", &self.args)
            .finish()
    }
}

impl Localize<'static> {
    pub fn new<'a>(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            args: HashMap::new(),
        }
    }

    #[must_use]
    pub fn arg(mut self, key: &str, value: impl IntoValue<FluentValue<'static>>) -> Self
    {
        self.args.insert(key.to_owned(), value.into_value());
        self
    }

    fn get_args(&self, context: &WidgetContext<'_>) -> FluentArgs {
        let mut res = FluentArgs::new();
        for (name, arg) in &self.args {
            res.set(name.to_owned(), arg.get_tracking_invalidate(context));
        }
        res
    }

    pub fn into_label(self) -> Label<Self> {
        Label::new(self)
    }
}

impl DynamicDisplay for Localize<'static> {
    fn generation(&self, context: &WidgetContext<'_>) -> Option<Generation> {
        context.locale().generation()
    }

    fn fmt(&self, context: &WidgetContext<'_>, f: &mut Formatter<'_>) -> fmt::Result {
        let locale = context.locale().get();
        println!("{:?}", locale);

        let bundle = context.translation();

        let message = if let Some(msg) = bundle.get_message(&self.key) {
            msg
        } else {
            return f.write_str(&format!("No message. locale: {}, key: {}", locale, self.key))
        };

        let value = if let Some(value) = message.value() {
            value
        } else {
            return f.write_str(&format!("No value. locale: {}, key: {}", locale, self.key))
        };

        let mut err = vec![];
        let args = self.get_args(&context);
        let res = bundle.format_pattern(value, Some(&args), &mut err);

        if err.is_empty() {
            f.write_str(&res)
        } else {
            f.write_str(&format!("{} {{Error. locale: {}, key: {}, cause: {:?}}}", locale, self.key, res, err))
        }
    }
}


#[derive(Default)]
pub struct TranslationCollection {
    translations: HashMap<LanguageIdentifier, String>
}

#[derive(Default, Clone)]
pub struct Translations {
    inner: Arc<Dynamic<TranslationCollection>>
}

impl Translations {
    pub fn add(&self, language: LanguageIdentifier, ftl: String) {
        let mut inner = self.inner.lock();

        inner.translations.insert(language, ftl);
    }

    #[must_use]
    pub fn with(self, language: LanguageIdentifier, ftl: String) -> Self {
        self.add(language, ftl);
        self
    }
}

pub struct TranslationState {
    pub(crate) fallback_locale: LanguageIdentifier,
    pub(crate) loaded_translations: HashMap<LanguageIdentifier, FluentBundle<FluentResource>>
}


impl TranslationState {
    pub fn new() -> Self {
        Self {
            fallback_locale: LanguageIdentifier::default(),
            loaded_translations: HashMap::from([(
                LanguageIdentifier::default(),
                FluentBundle::new(vec![LanguageIdentifier::default()]),
            )])
        }
    }

    pub fn add_all(&mut self, translations: Translations) {
        for (&ref language, ftl) in translations.inner.lock().translations.iter() {
            let res = FluentResource::try_new(ftl.clone())
                .expect("Failed to parse translations as FTL");
            let bundle =
                self.loaded_translations.entry(language.clone()).or_insert_with(|| FluentBundle::new(vec![language.clone()]));
            bundle.add_resource(res).expect("Failed to add resource to bundle");
        };
        self.renegotiate_fallback_language()
    }

    pub fn renegotiate_fallback_language(&mut self) {
        let available = self
            .loaded_translations
            .keys()
            .filter(|&x| x != &LanguageIdentifier::default())
            .collect::<Vec<_>>();
        let locale = sys_locale::get_locale()
            .and_then(|l| l.parse().ok())
            .unwrap_or_else(|| available.first().copied().cloned().unwrap_or_default());
        let default = LanguageIdentifier::default();
        let default_ref = &default; // ???
        let languages = fluent_langneg::negotiate::negotiate_languages(
            &[locale],
            &available,
            Some(&default_ref),
            fluent_langneg::NegotiationStrategy::Filtering,
        );
        self.fallback_locale = (**languages.first().unwrap()).clone();
    }
}