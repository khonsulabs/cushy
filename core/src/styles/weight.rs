use stylecs::{Points, UnscaledStyleComponent};

/// The weight of a font.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Weight {
    /// The thinnest font variant.
    Thin,
    /// The second-thinnest font variant. Thinner than `Light`, bolder than
    /// `Thin`.
    ExtraLight,
    /// A lighter (thinner) font variant. Thinner than `Normal`, bolder than
    /// `ExtraLight`.
    Light,
    /// The normal font weight.
    Normal,
    /// A slightly-bold font variant. Bolder than `Normal`, lighter than
    /// `SemiBold`.
    Medium,
    /// A bolder font variant. Bolder than `Medium`, lighter than `Bold`.
    SemiBold,
    /// A bold font variant. Bolder than `SemiBold`, lighter than `ExtraBold`.
    Bold,
    /// An extra-bold font variant. Bolder than `Bold`, lighter than `Black`.
    ExtraBold,
    /// The boldest font variant. Bolder than `ExtraBold`.
    Black,
    /// A specific weight.
    Other(u16),
}

impl Default for Weight {
    fn default() -> Self {
        Self::Normal
    }
}

impl UnscaledStyleComponent<Points> for Weight {}

impl Weight {
    /// Converts the weight to a `u16` using standard CSS mappings.
    #[must_use]
    pub const fn to_number(self) -> u16 {
        match self {
            Self::Thin => 100,
            Self::ExtraLight => 200,
            Self::Light => 300,
            Self::Normal => 400,
            Self::Medium => 500,
            Self::SemiBold => 600,
            Self::Bold => 700,
            Self::ExtraBold => 800,
            Self::Black => 900,
            Self::Other(n) => n,
        }
    }
}
