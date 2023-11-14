use manyhow::{manyhow, Result};
use proc_macro2::TokenStream;
use quote_use::quote_use as quote;
mod animation;

#[manyhow(proc_macro_derive(LinearInterpolate))]
pub use animation::linear_interpolate;
