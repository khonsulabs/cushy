use fluent_bundle::FluentValue;
use fluent_bundle::types::FluentNumber;
use unic_langid::LanguageIdentifier;
use cushy::localization::Localize;
use cushy::widget::{IntoWidgetList, MakeWidget};
use cushy::{Application, Open, PendingApp, Run};
use cushy::value::{Dynamic, Source};
use cushy::widgets::label::{Displayable};

fn localized() -> impl MakeWidget {
    let element_in_default_locale = Localize::new("message-hello-world")
        .into_label()
        .contain();

    let specific_locale: LanguageIdentifier = "es-ES".parse().unwrap();
    let elements_in_specific_locale = Localize::new("message-hello-world")
        .into_label()
        .localized(specific_locale)
        .contain();

    let dynamic_locale: Dynamic<LanguageChoices> = Dynamic::default();
    let dynamic_message_label = Localize::new("message-hello-world")
        .into_label();

    let dynamic_language_selector = dynamic_locale
        .new_radio(LanguageChoices::EN_GB).labelled_by(Localize::new("language-en-gb").into_label())
        .and(dynamic_locale.new_radio(LanguageChoices::EN_US).labelled_by(Localize::new("language-en-us").into_label()))
        .and(dynamic_locale.new_radio(LanguageChoices::ES_ES).labelled_by(Localize::new("language-es-es").into_label()))
        .into_rows()
        .contain();

    let bananas_counter = Dynamic::new(0u32);

    let counter_elements = Localize::new("banana-counter-message")
        .arg("bananas_counter", bananas_counter.map_each(|value|
            FluentValue::Number(FluentNumber::from(value)))
        )
        .into_label()
        .and("+".into_button().on_click(bananas_counter.with_clone(|counter| {
            move |_| {
                let mut counter = counter.lock();
                counter.checked_add(1).inspect(|new_counter|{
                    *counter = *new_counter;
                });
            }
        })))
        .and("-".into_button().on_click(bananas_counter.with_clone(|counter| {
            move |_| {
                let mut counter = counter.lock();
                counter.checked_sub(1).inspect(|new_counter|{
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
        .localized(dynamic_locale.map_each(LanguageChoices::to_locale));

    element_in_default_locale
        .and(elements_in_specific_locale)
        .and(dynamic_container)
        .into_rows()
        .contain()
        .centered()
}

#[derive(Default, Eq, PartialEq, Debug, Clone, Copy)]
pub enum LanguageChoices {
    EN_GB,
    #[default]
    EN_US,
    ES_ES,
}

impl LanguageChoices {
    pub fn to_locale(&self) -> LanguageIdentifier {
        match self {
            LanguageChoices::EN_GB => "en-GB".parse().unwrap(),
            LanguageChoices::EN_US => "en-US".parse().unwrap(),
            LanguageChoices::ES_ES => "es-ES".parse().unwrap(),
        }
    }
}


fn main() -> cushy::Result {

    let mut app = PendingApp::default();

    // If you comment this block out, you can see the effect of having missing translation files.
    {
        let en_us_locale: LanguageIdentifier = "en-US".parse().unwrap();
        let en_gb_locale: LanguageIdentifier = "en-GB".parse().unwrap();
        let es_es_locale: LanguageIdentifier = "es-ES".parse().unwrap();

        let translations = app.cushy().translations();
        translations
            .add(en_us_locale, include_str!("assets/translations/en-US/hello.ftl").to_owned());
        translations
            .add(en_gb_locale, include_str!("assets/translations/en-GB/hello.ftl").to_owned());
        translations
            .add(es_es_locale, include_str!("assets/translations/es-ES/hello.ftl").to_owned());
    }

    let _window_handle = localized()
        .into_window()
        .open(&mut app)?;

    app.run()
}

#[test]
fn runs() {
    cushy::example!(localized).untested_still_frame();
}
