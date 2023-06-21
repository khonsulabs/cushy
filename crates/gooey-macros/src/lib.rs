use attribute_derive::Attribute;
use manyhow::manyhow;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, Path};

#[derive(Attribute, Debug)]
#[attribute(ident = widget)]
struct Widget {
    name: Option<Ident>,
    authority: Option<Ident>,
    #[attribute(optional)]
    no_default_style: bool,
    core: Option<Path>,
}

#[manyhow]
#[proc_macro_derive(Widget, attributes(widget))]
pub fn widget_derive(input: TokenStream) -> manyhow::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        ..
    } = syn::parse2(input)?;
    let Widget {
        name,
        authority,
        no_default_style,
        core,
    } = Widget::from_attributes(&attrs)?;

    let core = core.map_or_else(|| quote!(::gooey_core), |core| quote!(#core));

    let name = if let Some(name) = name {
        validate(&name)?
    } else {
        stylecs_shared::pascal_case_to_snake_case(ident.to_string()).map_err(|_| manyhow::error_message!(ident.span(), "An invalid character for a stylecs Identifier was found. A name must be manually provided for this type."))?
    };

    let name = if let Some(authority) = authority {
        let authority = validate(&authority)?;
        quote!(#core::style::static_name!(#authority, #name))
    } else {
        quote!(#core::style::static_name!(#name))
    };

    let base_style = if no_default_style {
        quote!()
    } else {
        quote!(
            impl<#generics> #core::BaseStyle for #ident<#generics> {
                fn base_style(&self, library: &#core::style::Library) -> #core::style::WidgetStyle {
                    #core::style::WidgetStyle::default()
                }
            }
        )
    };

    Ok(quote! {
        impl<#generics> #core::Widget for #ident<#generics> {
            fn name(&self) -> #core::style::Name {
                <Self as #core::StaticWidget>::static_name()
            }
        }

        impl<#generics> #core::StaticWidget for #ident<#generics> {
            fn static_name() -> #core::style::Name {
                static NAME: #core::style::StaticName = #name;
                NAME.to_name()
            }
        }

        #base_style
    })
}

fn validate(name: &Ident) -> manyhow::Result<String> {
    let location = name.span();
    let name = name.to_string();
    stylecs_shared::validate_identifier(&name)
        .map_err(|_| manyhow::error_message!(location, "invalid character in identifier"))?;
    Ok(name)
}
