use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{FnArg, ItemFn, Type};

pub fn main(_attr: TokenStream, item: TokenStream) -> manyhow::Result {
    let function = syn::parse2::<ItemFn>(item)?;

    let mut inputs = function.sig.inputs.iter();
    let Some(FnArg::Typed(input)) = inputs.next() else {
        manyhow::bail!(
            "the cushy::main fn must accept one of `&mut cushy::PendingApp` or `&mut cushy::App`"
        )
    };
    if inputs.next().is_some() {
        manyhow::bail!("the cushy::main fn can have one input")
    }
    let Type::Reference(reference) = &*input.ty else {
        manyhow::bail!(
            "the cushy::main fn must accept one of `&mut cushy::PendingApp` or `&mut cushy::App`"
        );
    };
    let Type::Path(path) = &*reference.elem else {
        manyhow::bail!(
            "the cushy::main fn must accept one of `&mut cushy::PendingApp` or `&mut cushy::App`"
        )
    };

    let body = function.block;
    let (result, body) = match path.path.segments.last() {
        Some(segment) if segment.ident == "App" => match function.sig.output {
            syn::ReturnType::Default => (
                quote!(()),
                quote!(::cushy::run(|#input| #body).expect("event loop startup")),
            ),
            syn::ReturnType::Type(_, ty) => {
                let pat = &input.pat;
                (
                    ty.to_token_stream(),
                    quote!(
                        let mut app = ::cushy::PendingApp::default();
                        app.on_startup(|#pat: &mut #path| -> #ty #body);
                        ::cushy::Run::run(app)
                    ),
                )
            }
        },
        Some(segment) if segment.ident == "PendingApp" => {
            let pat = &input.pat;
            let original_output = function.sig.output;
            let (output, return_error) = match &original_output {
                syn::ReturnType::Default => (quote!(::cushy::Result), TokenStream::default()),
                syn::ReturnType::Type(_, ty) => (ty.to_token_stream(), quote!(?)),
            };
            (
                output,
                quote!(
                    let mut __pending_app = #path::default();
                    let cushy = __pending_app.cushy().clone();
                    let _guard = cushy.enter_runtime();
                    let init = |#pat: &mut #path| #original_output #body;
                    init(&mut __pending_app)#return_error;
                    ::cushy::Run::run(__pending_app)?;
                    Ok(())
                ),
            )
        }
        _ => manyhow::bail!(
            "the cushy::main fn must accept one of `&mut cushy::PendingApp` or `&mut cushy::App`"
        ),
    };

    manyhow::ensure!(
        function.sig.asyncness.is_none(),
        "cushy::main does not support async"
    );
    manyhow::ensure!(
        function.sig.constness.is_none(),
        "cushy::main does not support const"
    );

    let fn_token = function.sig.fn_token;
    let name = function.sig.ident;
    let unsafety = function.sig.unsafety;

    Ok(quote! {
        #unsafety #fn_token #name() -> #result {
            #body
        }
    })
}
