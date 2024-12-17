use manyhow::{manyhow, Result};
use proc_macro2::TokenStream;
use quote_use::quote_use as quote;

#[cfg(test)]
macro_rules! expansion_snapshot {
    ($name:ident $($tokens:tt)*) => {
        #[test]
        fn $name() {
            expansion_snapshot!{$($tokens)*}
        }
    };
    (#[derive($fn:expr)]$($tokens:tt)*) => {{
        use insta::assert_snapshot;
        use prettyplease::unparse;
        use syn::{parse2, parse_quote};
        let input = parse_quote!($($tokens)*);
        let output = $fn(input).unwrap();
        match &parse2(output.clone()) {
            Ok(ok) => assert_snapshot!(unparse(ok)),
            Err(_) => panic!("{output}"),
        }
    }};
}

mod animation;
mod cushy_main;

#[manyhow(proc_macro_derive(LinearInterpolate))]
pub use animation::linear_interpolate;
/// This macro extracts the body of `main` and uses it as the closure for the `on_startup` for the
/// [`cushy::PendingApp`].
///
/// Lifetimes:
///
/// Due to how this macro works variables in the closure will be dropped at the end of the
/// method body, and *NOT* when the application terminates.
#[manyhow(proc_macro_attribute)]
pub use cushy_main::main;
