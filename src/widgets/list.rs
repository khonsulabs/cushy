//! A list of elements with optional item indicators.

use std::fmt::Debug;
use std::sync::Arc;

use nominals::{
    ArmenianLower, ArmenianUpper, Bengali, Cambodian, CjkDecimal, CjkEarthlyBranch,
    CjkHeavenlyStem, Decimal, Devanagari, DigitCollection, EasternArabic, Ethiopic, Georgian,
    GreekLower, GreekUpper, Gujarati, Gurmukhi, HangeulFormal, HangeulJamo, HangeulSyllable,
    HanjaFormal, HanjaInformal, Hebrew, HexLower, HexUpper, Hiragana, HiraganaIroha,
    JapaneseFormal, JapaneseInformal, Kannada, Katakana, KatakanaIroha, Lao, LetterLower,
    LetterUpper, Malayalam, Mongolian, Myanmar, NominalSystem, Oriya, Persian, RomanLower,
    RomanUpper, SimplifiedChineseFormal, SimplifiedChineseInformal, Tamil, Telugu, Thai, Tibetan,
    TraditionalChineseFormal, TraditionalChineseInformal,
};

use super::grid::GridWidgets;
use super::input::CowString;
use super::label::DynamicDisplay;
use super::{Grid, Label};
use crate::reactive::value::{IntoValue, MapEach, Source, Value};
use crate::styles::{Component, RequireInvalidation};
use crate::widget::{MakeWidget, MakeWidgetWithTag, WidgetInstance, WidgetList};

/// A list of items displayed with an optional item indicator.
pub struct List {
    style: Value<ListStyle>,
    children: Value<WidgetList>,
}

impl List {
    /// Returns a new list with the default [`ListStyle`].
    #[must_use]
    pub fn new(children: impl IntoValue<WidgetList>) -> Self {
        Self {
            children: children.into_value(),
            style: Value::Constant(ListStyle::default()),
        }
    }

    /// Sets the style of list identifiers to `style`.
    #[must_use]
    pub fn style(mut self, style: impl IntoValue<ListStyle>) -> Self {
        self.style = style.into_value();
        self
    }
}

/// The style of a [`List`] widget's item indicators.
#[derive(Default, Debug, Clone)]
pub enum ListStyle {
    /// This list should have no indicators.
    None,

    /// A solid circle indicator, using the unicode bullet indicator.
    #[default]
    Disc,
    /// A hollow circle.
    Circle,
    /// A filled square.
    Square,

    /// Decimal digits (0-9).
    Decimal,
    /// Eastern Arabic digits.
    EasternArabic,
    /// Persian digits.
    Persian,

    /// Lowercase Armenian numbering.
    ArmenianLower,
    /// Uppercase Armenian numbering.
    ArmenianUpper,
    /// Bengali numeric digits.
    Bengali,
    /// Cambodian numeric digits.
    Cambodian,
    /// CJK Han decimal digits.
    CjkDecimal,
    /// CJK Earthly Branch symbols.
    ///
    /// This digit collection back to [`CjkDecimal`] after the set is enumerated.
    CjkEarthlyBranch,
    /// CJK Heavenly Stems symbols.
    ///
    /// This digit collection falls back to [`CjkDecimal`] after the set is
    /// enumerated.
    CjkHeavenlyStem,
    /// Devanagari numeric digits.
    Devanagari,
    /// Ethiopic numerical system.
    Ethiopic,
    /// Traditional Georgian numbering.
    Georgian,
    /// Gujarati numeric digits.
    Gujarati,
    /// Gurmukhi numeric digits.
    Gurmukhi,
    /// Korean Hangeul numbering.
    HangeulFormal,
    /// Informal Korean Hangeul numbering.
    HanjaInformal,
    /// Formal Korean Hanja numbering.
    HanjaFormal,
    /// Formal Japanese Kanji numbering.
    JapaneseFormal,
    /// Informal Japanese Kanji numbering.
    JapaneseInformal,
    /// Kannada numeric digits.
    Kannada,
    /// Lao numeric digits.
    Lao,
    /// Malayalam numeric digits.
    Malayalam,
    /// Mongolian numeric digits.
    Mongolian,
    /// Myanmar numeric digits.
    Myanmar,
    /// Oriya numeric digits.
    Oriya,
    /// Tamil numeric digits.
    Tamil,
    /// Telugu numeric digits.
    Telugu,
    /// Thai numeric digits.
    Thai,
    /// Tibetan numeric digits.
    Tibetan,

    /// ASCII lowercase alphabet (a-z).
    LetterLower,
    /// ASCII uppercase alphabet (A-Z).
    LetterUpper,
    /// Hexadecimal lowercase digits (0-9a-f)
    HexLower,
    /// Hexadecimal uppercase digits (0-9A-F)
    HexUpper,
    /// Informal Traditional Chinese with ordinary characters.
    ChineseTraditional,
    /// Informal Traditional Chinese with financial characters.
    ChineseTraditionalFinancial,
    /// Formal Traditional Chinese with ordinary characters.
    ChineseTraditionalFormal,
    /// Formal Traditional Chinese with financial characters.
    ChineseTraditionalFormalFinancial,
    /// Informal Simplified Chinese with ordinary characters.
    ChineseSimplified,
    /// Informal Simplified Chinese with financial characters.
    ChineseSimplifiedFinancial,
    /// Formal Simplified Chinese with ordinary characters.
    ChineseSimplifiedFormal,
    /// Formal Simplified Chinese with financial characters.
    ChineseSimplifiedFormalFinancial,
    /// Greek lowercase alphabet.
    GreekUpper,
    /// Greek uppercase alphabet.
    GreekLower,
    /// Japanese Hiragana Aiueo alphabet.
    Hiragana,
    /// Japanese Hiragana Iroha alphabet.
    HiraganaIroha,
    /// Japanese Katakana Aiueo alphabet.
    Katakana,
    /// Japanese Katakana Iroha alphabet.
    KatakanaIroha,
    /// Korean Hangeul Jamo alphabet.
    HangeulJamo,
    /// Korean Hangeul Syllable alphabet.
    HangeulSyllable,

    /// Lowercase Roman numerals (i, ii, iii, iv, ...).
    RomanLower,
    /// Uppercase Roman numerals (I, II, III, IV, ...).
    RomanUpper,
    /// Hebrew numerals.
    Hebrew,

    /// A custom list indicator style.
    Custom(Arc<dyn ListIndicator>),
}

impl ListStyle {
    /// Returns an iterator containing all built-in list styles.
    #[must_use]
    pub const fn provided() -> impl IntoIterator<Item = ListStyle> {
        [
            ListStyle::None,
            ListStyle::Disc,
            ListStyle::Circle,
            ListStyle::Square,
            ListStyle::Decimal,
            ListStyle::ArmenianLower,
            ListStyle::ArmenianUpper,
            ListStyle::Bengali,
            ListStyle::Cambodian,
            ListStyle::ChineseSimplified,
            ListStyle::ChineseSimplifiedFinancial,
            ListStyle::ChineseSimplifiedFormal,
            ListStyle::ChineseSimplifiedFormalFinancial,
            ListStyle::ChineseTraditional,
            ListStyle::ChineseTraditionalFinancial,
            ListStyle::ChineseTraditionalFormal,
            ListStyle::ChineseTraditionalFormalFinancial,
            ListStyle::CjkDecimal,
            ListStyle::CjkEarthlyBranch,
            ListStyle::CjkHeavenlyStem,
            ListStyle::Devanagari,
            ListStyle::EasternArabic,
            ListStyle::Ethiopic,
            ListStyle::Georgian,
            ListStyle::GreekLower,
            ListStyle::GreekUpper,
            ListStyle::Gujarati,
            ListStyle::Gurmukhi,
            ListStyle::HangeulFormal,
            ListStyle::HanjaInformal,
            ListStyle::HangeulJamo,
            ListStyle::HangeulSyllable,
            ListStyle::HanjaFormal,
            ListStyle::Hebrew,
            ListStyle::HexLower,
            ListStyle::HexUpper,
            ListStyle::Hiragana,
            ListStyle::HiraganaIroha,
            ListStyle::JapaneseFormal,
            ListStyle::JapaneseInformal,
            ListStyle::Kannada,
            ListStyle::Katakana,
            ListStyle::KatakanaIroha,
            ListStyle::Lao,
            ListStyle::LetterLower,
            ListStyle::LetterUpper,
            ListStyle::Malayalam,
            ListStyle::Mongolian,
            ListStyle::Myanmar,
            ListStyle::Oriya,
            ListStyle::Persian,
            ListStyle::RomanLower,
            ListStyle::RomanUpper,
            ListStyle::Tamil,
            ListStyle::Telugu,
            ListStyle::Thai,
            ListStyle::Tibetan,
        ]
    }
}

impl PartialEq for ListStyle {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// A [`ListStyle`] implementation that provides an optional indicator for a
/// given list index.
pub trait ListIndicator: Debug + Sync + Send + 'static {
    /// Returns the indicator to use at `index`.
    fn list_indicator(&self, index: usize) -> Option<Indicator>;
}

impl ListIndicator for ListStyle {
    #[allow(clippy::too_many_lines)] // can't avoid the match
    fn list_indicator(&self, index: usize) -> Option<Indicator> {
        match self {
            ListStyle::None => None,
            ListStyle::Decimal => Some(Indicator::delimited(String::from(
                Decimal.one_based().format_nominal(index),
            ))),
            ListStyle::Disc => Some(Indicator::bare(CowString::new("\u{2022}"))),
            ListStyle::Circle => Some(Indicator::bare(CowString::new("\u{25E6}"))),
            ListStyle::Square => Some(Indicator::bare(CowString::new("\u{25AA}"))),
            ListStyle::Custom(style) => style.list_indicator(index),
            ListStyle::EasternArabic => Some(Indicator::delimited(String::from(
                EasternArabic.one_based().format_nominal(index),
            ))),
            ListStyle::Persian => Some(Indicator::delimited(String::from(
                Persian.one_based().format_nominal(index),
            ))),
            ListStyle::LetterLower => Some(Indicator::delimited(String::from(
                LetterLower.one_based().format_nominal(index),
            ))),
            ListStyle::LetterUpper => Some(Indicator::delimited(String::from(
                LetterUpper.one_based().format_nominal(index),
            ))),
            ListStyle::HexLower => Some(Indicator::delimited(String::from(
                HexLower.one_based().format_nominal(index),
            ))),
            ListStyle::HexUpper => Some(Indicator::delimited(String::from(
                HexUpper.one_based().format_nominal(index),
            ))),
            ListStyle::GreekUpper => Some(Indicator::delimited(String::from(
                GreekUpper.one_based().format_nominal(index),
            ))),
            ListStyle::GreekLower => Some(Indicator::delimited(String::from(
                GreekLower.one_based().format_nominal(index),
            ))),
            ListStyle::Hiragana => Some(Indicator::delimited(String::from(
                Hiragana.one_based().format_nominal(index),
            ))),
            ListStyle::HiraganaIroha => Some(Indicator::delimited(String::from(
                HiraganaIroha.one_based().format_nominal(index),
            ))),
            ListStyle::Katakana => Some(Indicator::delimited(String::from(
                Katakana.one_based().format_nominal(index),
            ))),
            ListStyle::KatakanaIroha => Some(Indicator::delimited(String::from(
                KatakanaIroha.one_based().format_nominal(index),
            ))),
            ListStyle::HangeulJamo => Some(Indicator::delimited(String::from(
                HangeulJamo.one_based().format_nominal(index),
            ))),
            ListStyle::HangeulSyllable => Some(Indicator::delimited(String::from(
                HangeulSyllable.one_based().format_nominal(index),
            ))),
            ListStyle::RomanLower => Some(Indicator::delimited(String::from(
                RomanLower.format_nominal(index),
            ))),
            ListStyle::RomanUpper => Some(Indicator::delimited(String::from(
                RomanUpper.format_nominal(index),
            ))),
            ListStyle::Hebrew => Some(Indicator::delimited(String::from(
                Hebrew.format_nominal(index),
            ))),
            ListStyle::ArmenianLower => Some(Indicator::delimited(String::from(
                ArmenianLower.format_nominal(index),
            ))),
            ListStyle::ArmenianUpper => Some(Indicator::delimited(String::from(
                ArmenianUpper.format_nominal(index),
            ))),
            ListStyle::Bengali => Some(Indicator::delimited(String::from(
                Bengali.one_based().format_nominal(index),
            ))),
            ListStyle::Cambodian => Some(Indicator::delimited(String::from(
                Cambodian.one_based().format_nominal(index),
            ))),
            ListStyle::CjkDecimal => Some(Indicator::delimited(String::from(
                CjkDecimal.one_based().format_nominal(index),
            ))),
            ListStyle::CjkEarthlyBranch => Some(Indicator::delimited(String::from(
                CjkEarthlyBranch.one_based().format_nominal(index),
            ))),
            ListStyle::CjkHeavenlyStem => Some(Indicator::delimited(String::from(
                CjkHeavenlyStem.one_based().format_nominal(index),
            ))),
            ListStyle::Devanagari => Some(Indicator::delimited(String::from(
                Devanagari.one_based().format_nominal(index),
            ))),
            ListStyle::Ethiopic => Some(Indicator::delimited(String::from(
                Ethiopic.format_nominal(index),
            ))),
            ListStyle::Georgian => Some(Indicator::delimited(String::from(
                Georgian.format_nominal(index),
            ))),
            ListStyle::Gujarati => Some(Indicator::delimited(String::from(
                Gujarati.one_based().format_nominal(index),
            ))),
            ListStyle::Gurmukhi => Some(Indicator::delimited(String::from(
                Gurmukhi.one_based().format_nominal(index),
            ))),
            ListStyle::HangeulFormal => Some(Indicator::delimited(String::from(
                HangeulFormal.format_nominal(index),
            ))),
            ListStyle::HanjaInformal => Some(Indicator::delimited(String::from(
                HanjaInformal.format_nominal(index),
            ))),
            ListStyle::HanjaFormal => Some(Indicator::delimited(String::from(
                HanjaFormal.format_nominal(index),
            ))),
            ListStyle::JapaneseFormal => Some(Indicator::delimited(String::from(
                JapaneseFormal.format_nominal(index),
            ))),
            ListStyle::JapaneseInformal => Some(Indicator::delimited(String::from(
                JapaneseInformal.format_nominal(index),
            ))),
            ListStyle::Kannada => Some(Indicator::delimited(String::from(
                Kannada.one_based().format_nominal(index),
            ))),
            ListStyle::Lao => Some(Indicator::delimited(String::from(
                Lao.one_based().format_nominal(index),
            ))),
            ListStyle::Malayalam => Some(Indicator::delimited(String::from(
                Malayalam.one_based().format_nominal(index),
            ))),
            ListStyle::Mongolian => Some(Indicator::delimited(String::from(
                Mongolian.one_based().format_nominal(index),
            ))),
            ListStyle::Myanmar => Some(Indicator::delimited(String::from(
                Myanmar.one_based().format_nominal(index),
            ))),
            ListStyle::Oriya => Some(Indicator::delimited(String::from(
                Oriya.one_based().format_nominal(index),
            ))),
            ListStyle::Tamil => Some(Indicator::delimited(String::from(
                Tamil.one_based().format_nominal(index),
            ))),
            ListStyle::Telugu => Some(Indicator::delimited(String::from(
                Telugu.one_based().format_nominal(index),
            ))),
            ListStyle::Thai => Some(Indicator::delimited(String::from(
                Thai.one_based().format_nominal(index),
            ))),
            ListStyle::Tibetan => Some(Indicator::delimited(String::from(
                Tibetan.one_based().format_nominal(index),
            ))),
            ListStyle::ChineseTraditional => Some(Indicator::delimited(String::from(
                TraditionalChineseInformal::default().format_nominal(index),
            ))),
            ListStyle::ChineseTraditionalFinancial => Some(Indicator::delimited(String::from(
                TraditionalChineseInformal::default()
                    .financial()
                    .format_nominal(index),
            ))),
            ListStyle::ChineseTraditionalFormal => Some(Indicator::delimited(String::from(
                TraditionalChineseFormal::default().format_nominal(index),
            ))),
            ListStyle::ChineseTraditionalFormalFinancial => {
                Some(Indicator::delimited(String::from(
                    TraditionalChineseFormal::default()
                        .financial()
                        .format_nominal(index),
                )))
            }
            ListStyle::ChineseSimplified => Some(Indicator::delimited(String::from(
                SimplifiedChineseInformal::default().format_nominal(index),
            ))),
            ListStyle::ChineseSimplifiedFinancial => Some(Indicator::delimited(String::from(
                SimplifiedChineseInformal::default()
                    .financial()
                    .format_nominal(index),
            ))),
            ListStyle::ChineseSimplifiedFormal => Some(Indicator::delimited(String::from(
                SimplifiedChineseFormal::default().format_nominal(index),
            ))),
            ListStyle::ChineseSimplifiedFormalFinancial => {
                Some(Indicator::delimited(String::from(
                    SimplifiedChineseFormal::default()
                        .financial()
                        .format_nominal(index),
                )))
            }
        }
    }
}

impl MakeWidgetWithTag for List {
    fn make_with_tag(self, tag: crate::widget::WidgetTag) -> WidgetInstance {
        let rows = match (self.children, self.style) {
            (children, Value::Constant(style)) => {
                children.map_each(move |children| build_grid_widgets(&style, children))
            }
            (Value::Dynamic(children), Value::Dynamic(style)) => Value::Dynamic(
                (&style, &children)
                    .map_each(|(style, children)| build_grid_widgets(style, children)),
            ),
            (Value::Constant(children), Value::Dynamic(style)) => {
                Value::Dynamic(style.map_each(move |style| build_grid_widgets(style, &children)))
            }
        };
        Grid::from_rows(rows).make_with_tag(tag)
    }
}

fn build_grid_widgets(style: &ListStyle, children: &WidgetList) -> GridWidgets<2> {
    // This is horrible. We should be be using synchronize_with to avoid
    // recreating the gridwidgets every time.
    children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            (
                Label::new(
                    style
                        .list_indicator(index.wrapping_add(1))
                        .unwrap_or_default(),
                )
                .align_right()
                .align_top(),
                child.clone().align_left().make_widget(),
            )
        })
        .collect()
}

/// An indicator used in a [`List`] widget.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Indicator {
    display: CowString,
    delimited: bool,
}

impl Indicator {
    /// Returns an indicator that should show a [`Delimiter`] between itself and
    /// the list item.
    pub fn delimited(display: impl Into<CowString>) -> Self {
        Self {
            display: display.into(),
            delimited: true,
        }
    }

    /// Returns an indicator that skips rendering a [`Delimiter`].
    pub fn bare(display: impl Into<CowString>) -> Self {
        Self {
            display: display.into(),
            delimited: false,
        }
    }
}

impl DynamicDisplay for Indicator {
    fn fmt(
        &self,
        context: &crate::context::WidgetContext<'_>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let prefix = context.get(&Prefix);
        if self.delimited {
            let delimiter = context.get(&TrailingDelimiter);
            write!(f, "{}{}{}", prefix.0, self.display, delimiter.0)
        } else {
            write!(f, "{}{}", prefix.0, self.display)
        }
    }
}

/// A [`CowString`] type used in [`List`] widget style components.
#[derive(Default)]
pub struct ListDelimiter(CowString);

impl From<&'_ str> for ListDelimiter {
    fn from(value: &'_ str) -> Self {
        Self(value.into())
    }
}

impl From<ListDelimiter> for Component {
    fn from(value: ListDelimiter) -> Self {
        Component::String(value.0)
    }
}

impl TryFrom<Component> for ListDelimiter {
    type Error = Component;

    fn try_from(value: Component) -> Result<Self, Self::Error> {
        CowString::try_from(value).map(Self)
    }
}

impl RequireInvalidation for ListDelimiter {
    fn requires_invalidation(&self) -> bool {
        true
    }
}

define_components! {
    List {
        /// The delimiter to place between nested lists when using merged list
        /// indicators.
        Delimiter(ListDelimiter, "delimiter", ".".into())
        /// The delimiter to place after the list indictor, when the list is
        /// ordered.
        TrailingDelimiter(ListDelimiter, "trailing_delimiter", @Delimiter)
        /// The prefix to display before the list indicator.
        Prefix(ListDelimiter, "prefix")
    }
}
