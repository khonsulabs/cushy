use unic_langid::LanguageIdentifier;
use cushy::localization::{Localize, Translations};
use cushy::widget::{IntoWidgetList, MakeWidget};
use cushy::{Application, Open, PendingApp, Run};
use cushy::widgets::label::{Displayable};
use cushy::widgets::localized::Localized;

fn localized() -> impl MakeWidget {
    let element_in_default_locale = Localize::new("message-hello-world")
        .into_label();

    let specific_locale: LanguageIdentifier = "es-ES".parse().unwrap();
    let element_in_specific_local = Localize::new("message-hello-world")
        .into_label();

    let elements_in_specific_locale = Localized::new(specific_locale, element_in_specific_local);

    element_in_default_locale
        .and(elements_in_specific_locale)
        .into_rows()
}

fn main() -> cushy::Result {

    let mut app = PendingApp::default();

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

    let _window_handle = localized()
        .into_window()
        .open(&mut app)?;

    app.run()
}

#[test]
fn runs() {
    cushy::example!(localized).untested_still_frame();
}
