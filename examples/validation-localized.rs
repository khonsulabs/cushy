//! An example that demonstrates localized validation messages and errors.
//!
//! Note how all the form elements, hints and validation error messages use `localize!`
//!
//! Refer to the `localization.rs` example for more details on how localization works in general.

use unic_langid::LanguageIdentifier;
use cushy::figures::units::Lp;
use cushy::reactive::value::{Destination, Dynamic, IntoValue, Source, Validations, Value};
use cushy::widget::MakeWidget;
use cushy::widgets::input::InputValue;
use cushy::{localize, MaybeLocalized, Open, PendingApp};
use cushy::localization::Localization;

// This example is based on the `validation.rs` example and should be kept in sync with it.

fn form() -> impl MakeWidget {
    let text = Dynamic::default();
    let validations = Validations::default();

    localize!("form-example-hinted-label")
        .and(
            text.to_input()
                .validation(validations.validate(&text, validate_input))
                .hint(localize!("form-hint-field-required")),
        )
        .and(localize!("form-example-not-hinted-label"))
        .and(
            text.to_input()
                .validation(validations.validate(&text, validate_input)),
        )
        .and(
            localize!("form-generic-submit-button")
                .into_button()
                .on_click(validations.clone().when_valid(move |_| {

                    // Note: This is non-localized string is for developers, not users of the UI.
                    println!(
                        "Success! This callback only happens when all associated validations are valid"
                    );
                })),
        )
        .and(localize!("form-generic-reset-button").into_button().on_click(move |_| {
            let _value = text.take();
            validations.reset();
        }))
        .into_rows()
        .pad()
        .width(Lp::inches(6))
        .centered()
        .make_widget()
}

#[allow(clippy::ptr_arg)] // Changing &String to &str breaks type inference
fn validate_input(input: &String) -> Result<(), Value<MaybeLocalized>> {
    if input.is_empty() {
        Err(localize!("form-generic-invalid-empty").into_value())
    } else if input.trim().is_empty() {
        Err(localize!("form-input-invalid-non-whitespace-required").into_value())
    } else {
        Ok(())
    }
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum LanguageChoices {
    #[default]
    EnUs,
    EsEs,
}

impl LanguageChoices {
    pub fn to_locale(&self) -> LanguageIdentifier {
        match self {
            LanguageChoices::EnUs => "en-US".parse().unwrap(),
            LanguageChoices::EsEs => "es-ES".parse().unwrap(),
        }
    }
}

#[cushy::main]
fn main(app: &mut PendingApp) -> cushy::Result {
    app.cushy().localizations().add_default(
        Localization::for_language(
            "en-US",
            include_str!("assets/localizations/en-US/validation-localized.ftl"),
        )
            .expect("valid language id"),
    );
    app.cushy().localizations().add(
        Localization::for_language(
            "es-ES",
            include_str!("assets/localizations/es-ES/validation-localized.ftl"),
        )
            .expect("valid language id"),
    );

    let dynamic_locale: Dynamic<LanguageChoices> = Dynamic::default();

    ui(&dynamic_locale)
        .localized_in(dynamic_locale.map_each(LanguageChoices::to_locale))
        .into_window()
        .titled(localize!("window-title"))
        .open(app)?;

    Ok(())
}

fn language_selector(dynamic_locale: &Dynamic<LanguageChoices>) -> impl MakeWidget {
    let dynamic_language_selector = dynamic_locale
        .new_radio(LanguageChoices::EnUs)
        .labelled_by(localize!("language-en-us"))
        .and(
            dynamic_locale
                .new_radio(LanguageChoices::EsEs)
                .labelled_by(localize!("language-es-es")),
        )
        .into_rows()
        .contain();

    dynamic_language_selector
        .make_widget()
}

fn ui(dynamic_locale: &Dynamic<LanguageChoices>) -> impl MakeWidget {
    language_selector(dynamic_locale)
        .and(form())
        .into_rows()
        .centered()
        .make_widget()
}
