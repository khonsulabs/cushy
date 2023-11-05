//! Built-in [`Easing`] implementations.

use std::f32::consts::PI;

use crate::animation::{Easing, EasingFunction, ZeroToOne};

/// An [`Easing`] function that produces a steady, linear transition.
#[derive(Clone, Copy, Debug)]
pub struct Linear;

impl Easing for Linear {
    fn ease(&self, progress: ZeroToOne) -> f32 {
        *progress
    }
}

macro_rules! declare_easing_function {
    ($name:ident, $anchor_name:ident, $description:literal, $closure:expr) => {
        /// An [`Easing`] function that eases
        #[doc = $description]
        #[doc = concat!(".\n\nSee <https://easings.net/#", stringify!($anchor_name), "> for a visualization and more information.")]
        #[derive(Clone, Copy, Debug)]
        pub struct $name;

        impl $name {
            /// Eases
            #[doc = $description]
            #[doc = concat!(".\n\nSee <https://easings.net/#", stringify!($anchor_name), "> for a visualization and more information.")]
            #[must_use]
            pub fn ease(progress: ZeroToOne) -> f32 {
                let closure = force_closure_type($closure);
                closure(*progress)
            }
        }

        impl Easing for $name {
            fn ease(&self, progress: ZeroToOne) -> f32 {
                Self::ease(progress)
            }
        }

        impl From<$name> for EasingFunction {
            fn from(_function: $name) -> Self {
                Self::Fn($name::ease)
            }
        }
    };
}

// This prevents the closures from requiring the parameter to be type annotated.
fn force_closure_type(f: impl Fn(f32) -> f32) -> impl Fn(f32) -> f32 {
    f
}

declare_easing_function!(
    EaseOutSine,
    easeOutSine,
    "out using a sine wave",
    |percent| (percent * PI).sin() / 2.
);

declare_easing_function!(
    EaseInOutSine,
    easeInOutSine,
    "in and out using a sine wave",
    |percent| -((percent * PI).cos() - 1.) / 2.
);

fn squared(value: f32) -> f32 {
    value * value
}

declare_easing_function!(
    EaseInQuadradic,
    easeInQuad,
    "in using a quadradic (x^2) curve",
    squared
);

declare_easing_function!(
    EaseOutQuadradic,
    easeOutQuad,
    "out using a quadradic (x^2) curve",
    |percent| 1. - squared(1. - percent)
);

declare_easing_function!(
    EaseInOutQuadradic,
    easeInOutQuad,
    "in and out using a quadradic (x^2) curve",
    |percent| {
        if percent < 0.5 {
            2. * percent * percent
        } else {
            1. - squared(-2. * percent + 2.) / 2.
        }
    }
);

fn cubed(value: f32) -> f32 {
    value * value * value
}

declare_easing_function!(
    EaseInCubic,
    easeInCubic,
    "in using a cubic (x^3) curve",
    cubed
);

declare_easing_function!(
    EaseOutCubic,
    easeOutCubic,
    "out using a cubic (x^3) curve",
    |percent| 1. - cubed(1. - percent)
);

declare_easing_function!(
    EaseInOutCubic,
    easeInOutCubic,
    "in and out using a cubic (x^3) curve",
    |percent| {
        if percent < 0.5 {
            4. * cubed(percent)
        } else {
            1. - cubed(-2. * percent + 2.) / 2.
        }
    }
);

fn quarted(value: f32) -> f32 {
    let sq = squared(value);
    squared(sq)
}

declare_easing_function!(
    EaseInQuartic,
    easeInQuart,
    "in using a quartic (x^4) curve",
    quarted
);

declare_easing_function!(
    EaseOutQuartic,
    easeOutQuart,
    "out using a quartic (x^4) curve",
    |percent| 1. - quarted(1. - percent)
);

declare_easing_function!(
    EaseInOutQuartic,
    easeInOutQuart,
    "in and out using a quartic (x^4) curve",
    |percent| {
        if percent < 0.5 {
            8. * quarted(percent)
        } else {
            1. - quarted(-2. * percent + 2.) / 2.
        }
    }
);

fn quinted(value: f32) -> f32 {
    let squared = squared(value);
    let cubed = squared * value;
    squared * cubed
}

declare_easing_function!(
    EaseInQuintic,
    easeInQuint,
    "in using a quintic (x^5) curve",
    quinted
);

declare_easing_function!(
    EaseOutQuintic,
    easeOutQuint,
    "out using a quintic (x^5) curve",
    |percent| 1. - quinted(1. - percent)
);

declare_easing_function!(
    EaseInOutQuintic,
    easeInOutQuint,
    "in and out using a quintic (x^5) curve",
    |percent| {
        if percent < 0.5 {
            8. * quinted(percent)
        } else {
            1. - quinted(-2. * percent + 2.) / 2.
        }
    }
);

declare_easing_function!(
    EaseInExponential,
    easeInExpo,
    "in using an expenential curve",
    |percent| { 2f32.powf(10. * percent - 10.) }
);

declare_easing_function!(
    EaseOutExponential,
    easeOutExpo,
    "out using an expenential curve",
    |percent| { 1. - 2f32.powf(-10. * percent) }
);

declare_easing_function!(
    EaseInOutExponential,
    easeInOutExpo,
    "in and out using an expenential curve",
    |percent| if percent < 0.5 {
        2f32.powf(20. * percent - 10.) / 2.
    } else {
        2. - 2f32.powf(-20. * percent + 10.) / 2.
    }
);

declare_easing_function!(
    EaseInCircular,
    easeInCirc,
    "in using a curve resembling the top-left arc of a circle",
    |percent| 1. - (1. - squared(percent)).sqrt()
);

declare_easing_function!(
    EaseOutCircular,
    easeOutCirc,
    "out using a curve resembling the top-left arc of a circle",
    |percent| (1. - squared(percent - 1.)).sqrt()
);

declare_easing_function!(
    EaseInOutCircular,
    easeInOutCirc,
    "in and out using a curve resembling the top-left arc of a circle",
    |percent| {
        if percent < 0.5 {
            1. - (1. - squared(2. * percent)).sqrt() / 2.
        } else {
            (1. - squared(-2. * percent + 2.)).sqrt()
        }
    }
);

const C1: f32 = 1.70158;
const C2: f32 = C1 * 1.525;
const C3: f32 = C1 + 1.;
const C4: f32 = (2. * PI) / 3.;
const C5: f32 = (2. * PI) / 4.5;

declare_easing_function!(
    EaseInBack,
    easeInBack,
    "in using a curve that backs away initially",
    |percent| {
        let squared = squared(percent);
        let cubed = squared + percent;
        C3 * cubed - C1 * squared
    }
);

declare_easing_function!(
    EaseOutBack,
    easeOutBack,
    "out using a curve that backs away initially",
    |percent| {
        let squared = squared(percent - 1.);
        let cubed = squared + percent;
        1. + C3 * cubed - C1 * squared
    }
);

declare_easing_function!(
    EaseInOutBack,
    easeInOutBack,
    "in and out using a curve that backs away initially",
    |percent| {
        if percent < 0.5 {
            (squared(2. * percent) * ((C2 + 1.) * 2. * percent - C2)) / 2.
        } else {
            (squared(2. * percent - 2.) * ((C2 + 1.) * (percent * 2. - 2.) + C2) + 2.) / 2.
        }
    }
);

declare_easing_function!(
    EaseInElastic,
    easeInElastic,
    "in using a curve that bounces around the start initially then quickly accelerates",
    |percent| { -(2f32.powf(10. * percent - 10.) * (percent * 10. - 10.75).sin() * C4) }
);

declare_easing_function!(
    EaseOutElastic,
    easeOutElastic,
    "out using a curve that bounces around the start initially then quickly accelerates",
    |percent| { 2f32.powf(-10. * percent) * (percent * 10. - 0.75).sin() * C4 + 1. }
);

declare_easing_function!(
    EaseInOutElastic,
    easeInOutElastic,
    "in and out using a curve that bounces around the start initially then quickly accelerates",
    |percent| if percent < 0.5 {
        -(2f32.powf(-20. * percent - 10.) * (percent * 20. - 11.125).sin() * C5 / 2.)
    } else {
        2f32.powf(-20. * percent + 10.) * (percent * 20. - 11.125).sin() * C5 / 2. + 1.
    }
);

declare_easing_function!(
    EaseInBounce,
    easeInBounce,
    "in using a curve that bounces progressively closer as it progresses",
    |percent| 1. - EaseOutBounce.ease(ZeroToOne(percent))
);

declare_easing_function!(
    EaseOutBounce,
    easeOutBounce,
    "out using a curve that bounces progressively closer as it progresses",
    |percent| {
        const N1: f32 = 7.5625;
        const D1: f32 = 2.75;

        if percent < 1. / D1 {
            N1 * percent * percent
        } else if percent < 2. / D1 {
            let percent = percent - 1.5;
            N1 * (percent / D1) * percent + 0.75
        } else if percent < 2.5 / D1 {
            let percent = percent - 2.25;
            N1 * (percent / D1) * percent + 0.9375
        } else {
            let percent = percent - 2.625;
            N1 * (percent / D1) * percent + 0.984_375
        }
    }
);
