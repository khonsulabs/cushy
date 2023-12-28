use manyhow::{bail, ensure};
use quote::ToTokens;
use syn::{Data, DeriveInput, Field, Variant};

use crate::*;

pub fn linear_interpolate(
    DeriveInput {
        ident: item_ident,
        generics,
        data,
        ..
    }: DeriveInput,
) -> Result<TokenStream> {
    if let Some(generic) = generics.type_params().next() {
        bail!(generic, "generics not supported");
    }

    let doc;

    let body = match data {
        Data::Struct(data) => {
            let fields = match data.fields {
                syn::Fields::Unit => bail!(item_ident, "unit structs are not supported"),
                fields => fields
                    .into_iter()
                    .enumerate()
                    .map(|(idx, Field {  ident, .. })| {
                        let ident = ident
                            .map(ToTokens::into_token_stream)
                            .unwrap_or_else(|| proc_macro2::Literal::usize_unsuffixed(idx).into_token_stream());
                            quote!(#ident: ::cushy::animation::LinearInterpolate::lerp(&self.#ident, &__target.#ident, __percent),)
                    }),
            };
            doc = "# Panics\n Panics if any field's lerp panics (this should only happen on percentages outside 0..1 range).";
            quote!(#item_ident{#(#fields)*})
        }
        Data::Enum(data) => {
            let variants = data
                .variants
                .into_iter()
                .map(
                    |Variant {
                         ident,
                         fields,
                         discriminant,
                         ..
                     }| {
                        if let Some(discriminant) = discriminant {
                            bail!(discriminant, "discriminants are not supported");
                        }
                        ensure!(fields.is_empty(), fields, "enum fields are not supported");
                        Ok(quote!(#item_ident::#ident #fields))
                    },
                )
                .collect::<Result<Vec<_>>>()?;
            let last = variants
                .last()
                .map(ToTokens::to_token_stream)
                .unwrap_or_else(|| quote!(unreachable!()));

            let idx: Vec<_> = (0..variants.len()).collect();
            doc = "# Panics\n Panics if the the enum variants are overflown (this can only happen on percentages outside 0..1 range).";
            quote! {
                # use ::cushy::animation::LinearInterpolate;
                fn variant_to_index(__v: &#item_ident) -> usize {
                    match __v {
                        #(#variants => #idx,)*
                    }
                }
                let __self = variant_to_index(&self);
                let __target = variant_to_index(&__target);
                match LinearInterpolate::lerp(&__self, &__target, __percent) {
                    #(#idx => #variants,)*
                    _ => #last,
                }
            }
        }
        Data::Union(union) => bail!((union.union_token, union.fields), "unions not supported"),
    };

    Ok(quote! {
        impl ::cushy::animation::LinearInterpolate for #item_ident {
            #[doc = #doc]
            fn lerp(&self, __target: &Self, __percent: f32) -> Self {
                #body
            }
        }
    })
}

#[cfg(test)]
mod test {
    use super::*;
    expansion_snapshot! {struct_
        #[derive(linear_interpolate)]
        struct HelloWorld {
            fielda: Hello,
            fieldb: World,
        }
    }
    expansion_snapshot! {tuple_struct
        #[derive(linear_interpolate)]
        struct HelloWorld(Hello, World);
    }
    expansion_snapshot! {enum_
        #[derive(linear_interpolate)]
        enum Enum{A, B}
    }
    expansion_snapshot! {empty_enum
        #[derive(linear_interpolate)]
        enum Enum{}
    }
}
