use cushy::localization::{Localization, Localize};
use cushy::value::{Dynamic, Source};
use cushy::widget::MakeWidget;
use cushy::{Open, PendingApp};
use unic_langid::LanguageIdentifier;

fn localized() -> impl MakeWidget {
    let element_in_default_locale = Localize::new("message-hello-world").contain();

    let specific_locale: LanguageIdentifier = "es-ES".parse().unwrap();
    let elements_in_specific_locale = Localize::new("message-hello-world")
        .localized_in(specific_locale)
        .contain();

    let dynamic_locale: Dynamic<LanguageChoices> = Dynamic::default();
    let dynamic_message_label = Localize::new("message-hello-world");

    let dynamic_language_selector = dynamic_locale
        .new_radio(LanguageChoices::EnGb)
        .labelled_by(Localize::new("language-en-gb"))
        .and(
            dynamic_locale
                .new_radio(LanguageChoices::EnUs)
                .labelled_by(Localize::new("language-en-us")),
        )
        .and(
            dynamic_locale
                .new_radio(LanguageChoices::EsEs)
                .labelled_by(Localize::new("language-es-es")),
        )
        .into_rows()
        .contain();

    let bananas_counter = Dynamic::new(0u32);

    let counter_elements = Localize::new("banana-counter-message")
        .arg("bananas_counter", &bananas_counter)
        .and(
            "+".into_button()
                .on_click(bananas_counter.with_clone(|counter| {
                    move |_| {
                        let mut counter = counter.lock();
                        counter.checked_add(1).inspect(|new_counter| {
                            *counter = *new_counter;
                        });
                    }
                })),
        )
        .and(
            "-".into_button()
                .on_click(bananas_counter.with_clone(|counter| {
                    move |_| {
                        let mut counter = counter.lock();
                        counter.checked_sub(1).inspect(|new_counter| {
                            *counter = *new_counter;
                        });
                    }
                })),
        )
        .into_columns();

    let dynamic_container = dynamic_message_label
        .and(counter_elements)
        .and(dynamic_language_selector)
        .into_rows()
        .contain()
        .localized_in(dynamic_locale.map_each(LanguageChoices::to_locale));

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
    // If you comment this block out, you can see the effect of having missing translation files.
    {
        app.cushy().translations().add_default(
            Localization::for_language(
                "en-US",
                include_str!("assets/translations/en-US/hello.ftl"),
            )
            .expect("valid language id"),
        );
        app.cushy().translations().add(
            Localization::for_language(
                "en-GB",
                include_str!("assets/translations/en-GB/hello.ftl"),
            )
            .expect("valid language id"),
        );
        app.cushy().translations().add(
            Localization::for_language(
                "es-ES",
                include_str!("assets/translations/es-ES/hello.ftl"),
            )
            .expect("valid language id"),
        );
    }

    localized().into_window().open(app)?;

    Ok(())
}

#[test]
fn runs() {
    cushy::example!(localized).untested_still_frame();
}
