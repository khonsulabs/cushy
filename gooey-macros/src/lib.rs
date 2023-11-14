use manyhow::{manyhow, Result};
use quote_use::quote_use as quote;
use proc_macro2::TokenStream;
mod animation;

#[manyhow(proc_macro_derive(LinearInterpolate))]
pub use animation::linear_interpolate;
