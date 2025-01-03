use cushy::localization::{Localization, Localize};
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::{localize, Open, PendingApp};
use unic_langid::LanguageIdentifier;
use cushy::widgets::Localized;

fn localization() -> impl MakeWidget {
    // Create a widget showing `message-hello-world`, which we will place on the
    // window such that it detects the system locale.
    let element_in_default_locale = localize!("message-hello-world").contain();

    // Create a widget showing `message-hello-world` in Spanish, always.
    let specific_locale: LanguageIdentifier = "es-ES".parse().unwrap();
    let elements_in_specific_locale = localize!("message-hello-world")
        .localized_in(specific_locale)
        .contain();

    // Create a widget that shows `message-hello-world` in the locale selected
    // by this example's available locales.
    let dynamic_locale: Dynamic<LanguageChoices> = Dynamic::default();
    let dynamic_message_label = localize!("message-hello-world");

    let dynamic_language_selector = dynamic_locale
        .new_radio(LanguageChoices::EnGb)
        .labelled_by(localize!("language-en-gb"))
        .and(
            dynamic_locale
                .new_radio(LanguageChoices::EnUs)
                .labelled_by(localize!("language-en-us")),
        )
        .and(
            dynamic_locale
                .new_radio(LanguageChoices::EsEs)
                .labelled_by(localize!("language-es-es")),
        )
        .into_rows()
        .contain();

    // Fluent also supports parameterization, allowing localizers incredible
    // flexibility in how messages and values are localized. This example shows
    // how a dynamic counter can be used in localization in Cushy.
    let bananas = Dynamic::new(0u32);

    let counter_elements = localize!("banana-counter-message", "bananas" => &bananas)
        .and("+".into_button().on_click(bananas.with_clone(|counter| {
            move |_| {
                let mut counter = counter.lock();
                counter.checked_add(1).inspect(|new_counter| {
                    *counter = *new_counter;
                });
            }
        })))
        .and("-".into_button().on_click(bananas.with_clone(|counter| {
            move |_| {
                let mut counter = counter.lock();
                counter.checked_sub(1).inspect(|new_counter| {
                    *counter = *new_counter;
                });
            }
        })))
        .into_columns();

    let dynamic_container = dynamic_message_label
        .and(counter_elements)
        .and(dynamic_language_selector)
        .into_rows()
        .contain()
        .localized_in(dynamic_locale.map_each(LanguageChoices::to_locale));

    // Assemble the parts of the interface.
    element_in_default_locale
        .and(elements_in_specific_locale)
        .and(dynamic_container)
        .into_rows()
        .contain()
        .centered()
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum LanguageChoices {
    EnGb,
    #[default]
    EnUs,
    EsEs,
}

impl LanguageChoices {
    pub fn to_locale(&self) -> LanguageIdentifier {
        match self {
            LanguageChoices::EnGb => "en-GB".parse().unwrap(),
            LanguageChoices::EnUs => "en-US".parse().unwrap(),
            LanguageChoices::EsEs => "es-ES".parse().unwrap(),
        }
    }
}

#[cushy::main]
fn main(app: &mut PendingApp) -> cushy::Result {
    // If you comment this block out, you can see the effect of having missing localization files.
    {
        // Adds a localization for en-US, setting it as the default
        // localization. If the system running this application is not
        // compatible with the available locales, the `en-US` localization will
        // be used.
        app.cushy().localizations().add_default(
            Localization::for_language(
                "en-US",
                include_str!("assets/localizations/en-US/hello.ftl"),
            )
            .expect("valid language id"),
        );
        // Adds a localization for en-GB. Fluent supports region-specific
        // localizations, and Cushy will attempt to find localizations in the
        // best-matching locale.
        app.cushy().localizations().add(
            Localization::for_language(
                "en-GB",
                include_str!("assets/localizations/en-GB/hello.ftl"),
            )
            .expect("valid language id"),
        );
        // Adds a localization for es-ES.
        app.cushy().localizations().add(
            Localization::for_language(
                "es-ES",
                include_str!("assets/localizations/es-ES/hello.ftl"),
            )
            .expect("valid language id"),
        );
    }

    localization().into_window()
        .titled(localize!("window-title"))
        .open(app)?;

    Ok(())
}

#[test]
fn runs() {
    cushy::example!(localization).untested_still_frame();
}
