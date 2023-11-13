use attribute_derive::FromAttr;
use manyhow::bail;
use quote::ToTokens;
use syn::{Field, ItemStruct};

use crate::*;

#[derive(FromAttr)]
#[attribute(ident = interpolate)]
struct FieldAttributes {
    skip: bool,
}

pub fn linear_interpolate(
    ItemStruct {
        ident,
        generics,
        fields,
        ..
    }: ItemStruct,
) -> Result<TokenStream> {
    if let Some(generic) = generics.type_params().next() {
        bail!(generic, "generics not supported");
    }

    let fields = match fields {
        syn::Fields::Unit => bail!(ident, "unit structs are not supported"),
        fields => fields
            .into_iter()
            .enumerate()
            .map(|(idx, Field { attrs, ident, .. })| {
                let ident = ident
                    .map(ToTokens::into_token_stream)
                    .unwrap_or_else(|| proc_macro2::Literal::usize_unsuffixed(idx).into_token_stream());
                Ok(if FieldAttributes::from_attributes(&attrs)?.skip {
                    quote!(#ident: __target.#ident,)
                } else {
                    quote!(#ident: ::gooey::animation::LinearInterpolate::lerp(&self.#ident, &__target.#ident, __percent),)
                })
            }),
    }.collect::<Result>()?;

    Ok(quote! {
        impl ::gooey::animation::LinearInterpolate for #ident {
            fn lerp(&self, __target: &Self, __percent: f32) -> Self {
                #ident{#fields}
            }
        }
    })
}

#[test]
fn test() {
    use insta::assert_snapshot;
    use prettyplease::unparse;
    use syn::{parse2, parse_quote};
    let input = parse_quote!(
        struct HelloWorld {
            fielda: Hello,
            fieldb: World,
        }
    );
    let output = linear_interpolate(input).unwrap();
    assert_snapshot!(unparse(&parse2(output).unwrap()))
}
