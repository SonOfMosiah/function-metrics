use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprAssign, ExprField, ExprPath, GenericArgument, ItemFn, LitStr, Member, PathArguments, ReturnType, Token,
    Type, TypeParamBound,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

struct Label {
    key: String,
    value: LabelValue,
}

enum LabelValue {
    Static(LitStr),
    Dynamic(Expr),
}

impl Parse for Label {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let expression = input.parse::<Expr>()?;

        match expression {
            Expr::Assign(ExprAssign { left, right, .. }) => {
                let key = label_key(&left)?;
                let value = match *right {
                    Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(value),
                        ..
                    }) => LabelValue::Static(value),
                    expression => LabelValue::Dynamic(expression),
                };
                Ok(Self { key, value })
            }
            expression => {
                let key = label_key(&expression)?;
                Ok(Self {
                    key,
                    value: LabelValue::Dynamic(expression),
                })
            }
        }
    }
}

fn label_key(expression: &Expr) -> syn::Result<String> {
    let key = match expression {
        Expr::Path(ExprPath { path, .. }) => path
            .get_ident()
            .map(ToString::to_string)
            .ok_or_else(|| syn::Error::new_spanned(expression, "label must be a simple identifier")),
        Expr::Field(ExprField {
            member: Member::Named(member),
            ..
        }) => Ok(member.to_string()),
        Expr::Field(ExprField {
            member: Member::Unnamed(_),
            ..
        }) => Err(syn::Error::new_spanned(
            expression,
            "tuple fields need an explicit label name, such as `index = value.0`",
        )),
        _ => Err(syn::Error::new_spanned(
            expression,
            "label must be an identifier, field access, or `key = value`",
        )),
    }?;

    validate_snake_case_name(&key, expression, "label")?;
    Ok(key)
}

#[derive(Default)]
struct FunctionMetricsArgs {
    name: Option<LitStr>,
    labels: Vec<Label>,
}

impl Parse for FunctionMetricsArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut args = Self::default();
        let mut labels_seen = false;

        while !input.is_empty() {
            let option = input.parse::<syn::Ident>()?;
            match option.to_string().as_str() {
                "name" => {
                    if args.name.is_some() {
                        return Err(syn::Error::new(option.span(), "duplicate `name` option"));
                    }
                    input.parse::<Token![=]>()?;
                    args.name = Some(input.parse()?);
                }
                "labels" => {
                    if labels_seen {
                        return Err(syn::Error::new(option.span(), "duplicate `labels` option"));
                    }
                    labels_seen = true;
                    let content;
                    syn::parenthesized!(content in input);
                    args.labels = Punctuated::<Label, Token![,]>::parse_terminated(&content)?
                        .into_iter()
                        .collect();

                    let mut unique_keys = HashSet::new();
                    for label in &args.labels {
                        if !unique_keys.insert(&label.key) {
                            return Err(syn::Error::new(
                                option.span(),
                                format!("duplicate label key `{}`", label.key),
                            ));
                        }
                    }
                }
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "expected `name = \"operation\"` or `labels(...)`",
                    ));
                }
            }

            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }

        Ok(args)
    }
}

/// Instruments a function with a Prometheus-compatible duration histogram.
///
/// The operation name defaults to the Rust function name. The emitted
/// histogram is named `{operation}_duration_seconds`, and elapsed time is
/// recorded as fractional seconds through the `metrics` facade. Normal
/// returns, panics, and cancellation after polling begins all record a value.
/// `#[track_caller]` functions and non-async functions returning `impl Future`
/// are rejected because they cannot be instrumented without changing semantics.
/// Renamed Future imports and concrete future return-type aliases cannot be detected.
///
/// ```ignore
/// #[function_metrics]
/// async fn refresh_cache() {}
///
/// #[function_metrics(name = "handle_request", labels(method, service = "api"))]
/// async fn handle_request(method: Method) {}
/// ```
#[proc_macro_attribute]
pub fn function_metrics(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as FunctionMetricsArgs);
    let mut input_fn = parse_macro_input!(input as ItemFn);

    if let Some(constness) = input_fn.sig.constness {
        return syn::Error::new(constness.span, "`function_metrics` does not support const functions")
            .into_compile_error()
            .into();
    }
    if let Some(attribute) = input_fn
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("track_caller"))
    {
        return syn::Error::new_spanned(
            attribute,
            "`function_metrics` does not support `#[track_caller]`; instrumentation would change caller locations",
        )
        .into_compile_error()
        .into();
    }
    if input_fn.sig.asyncness.is_none() && returns_impl_future(&input_fn.sig.output) {
        return syn::Error::new_spanned(
            &input_fn.sig.output,
            "`function_metrics` does not support functions returning `impl Future`; use `async fn` so execution can be timed",
        )
        .into_compile_error()
        .into();
    }

    let operation_name = args
        .name
        .unwrap_or_else(|| LitStr::new(&input_fn.sig.ident.to_string(), input_fn.sig.ident.span()));
    if let Err(error) = validate_operation_name(&operation_name) {
        return error.into_compile_error().into();
    }

    let runtime = match runtime_crate_path() {
        Ok(runtime) => runtime,
        Err(error) => return error.into_compile_error().into(),
    };
    let histogram_name = LitStr::new(
        &format!("{}_duration_seconds", operation_name.value()),
        operation_name.span(),
    );

    let mut labels = Vec::new();
    for label in args.labels {
        let key = label.key;
        match label.value {
            LabelValue::Static(value) => {
                labels.push(quote! { #runtime::__private::Label::new(#key, #value) });
            }
            LabelValue::Dynamic(expression) => {
                labels.push(quote! { #runtime::__private::Label::new(#key, (#expression).to_string()) });
            }
        }
    }

    let original_block = &input_fn.block;
    let measure_sync = quote! {
        #runtime::__private::measure_sync(
            #histogram_name,
            ::std::vec![#(#labels),*],
            || #original_block,
        )
    };
    let measure_future = quote! {
        #runtime::__private::measure_future(
            #histogram_name,
            ::std::vec![#(#labels),*],
            async #original_block,
        )
    };

    let is_native_async = input_fn.sig.asyncness.is_some();
    let is_async_trait_future = !is_native_async && returns_future(&input_fn.sig.output);

    let new_block = if is_native_async {
        quote! {{
            #measure_future.await
        }}
    } else if is_async_trait_future {
        quote! {{
            ::std::boxed::Box::pin(async move {
                #runtime::__private::measure_future(
                    #histogram_name,
                    ::std::vec![#(#labels),*],
                    async move { (|| #original_block)().await },
                ).await
            })
        }}
    } else {
        quote! {{
            #measure_sync
        }}
    };

    input_fn.block = syn::parse2(new_block).expect("generated instrumentation block must parse");
    quote!(#input_fn).into()
}

fn runtime_crate_path() -> syn::Result<TokenStream2> {
    match crate_name("function-metrics") {
        Ok(FoundCrate::Itself) => Ok(quote!(::function_metrics)),
        Ok(FoundCrate::Name(name)) => {
            let name = format_ident!("{name}");
            Ok(quote!(::#name))
        }
        Err(error) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("could not locate the `function-metrics` facade crate: {error}"),
        )),
    }
}

fn validate_operation_name(name: &LitStr) -> syn::Result<()> {
    validate_snake_case_name(&name.value(), name, "operation")
}

fn validate_snake_case_name(value: &str, span: impl quote::ToTokens, kind: &str) -> syn::Result<()> {
    let valid_characters = value
        .chars()
        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_');
    let valid_start = value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_lowercase() || character == '_');

    if !valid_start || !valid_characters {
        return Err(syn::Error::new_spanned(
            span,
            format!("{kind} names must use Prometheus-compatible snake_case characters"),
        ));
    }

    Ok(())
}

fn returns_future(return_type: &ReturnType) -> bool {
    let ReturnType::Type(_, return_type) = return_type else {
        return false;
    };

    let Some(pin_inner) = single_type_argument(return_type, "Pin") else {
        return false;
    };
    let Some(box_inner) = single_type_argument(pin_inner, "Box") else {
        return false;
    };

    let Type::TraitObject(trait_object) = box_inner else {
        return false;
    };
    trait_object
        .bounds
        .iter()
        .any(|bound| matches!(bound, TypeParamBound::Trait(trait_bound) if is_standard_future_path(&trait_bound.path)))
}

fn returns_impl_future(return_type: &ReturnType) -> bool {
    let ReturnType::Type(_, return_type) = return_type else {
        return false;
    };
    let Type::ImplTrait(impl_trait) = return_type.as_ref() else {
        return false;
    };

    impl_trait.bounds.iter().any(|bound| {
        let TypeParamBound::Trait(trait_bound) = bound else {
            return false;
        };
        trait_bound
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Future")
    })
}

fn is_standard_future_path(path: &syn::Path) -> bool {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    matches!(segments.as_slice(), [root, module, name] if (root == "core" || root == "std") && module == "future" && name == "Future")
}

fn single_type_argument<'a>(ty: &'a Type, expected_type: &str) -> Option<&'a Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != expected_type {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    let mut type_arguments = arguments.args.iter().filter_map(|argument| match argument {
        GenericArgument::Type(ty) => Some(ty),
        _ => None,
    });
    let argument = type_arguments.next()?;
    type_arguments.next().is_none().then_some(argument)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_label_options_even_when_the_first_is_empty() {
        let error = syn::parse_str::<FunctionMetricsArgs>("labels(), labels(method)")
            .err()
            .expect("duplicate labels should fail");
        assert!(error.to_string().contains("duplicate `labels` option"));
    }

    #[test]
    fn rejects_duplicate_label_keys() {
        let error = syn::parse_str::<FunctionMetricsArgs>("labels(method, method)")
            .err()
            .expect("duplicate keys should fail");
        assert!(error.to_string().contains("duplicate label key `method`"));
    }

    #[test]
    fn rejects_tuple_fields_without_an_explicit_key() {
        let error = syn::parse_str::<FunctionMetricsArgs>("labels(request.0)")
            .err()
            .expect("unnamed fields should fail");
        assert!(error.to_string().contains("explicit label name"));
    }

    #[test]
    fn rejects_non_snake_case_operation_names() {
        let name = LitStr::new("RequestDuration", proc_macro2::Span::call_site());
        assert!(validate_operation_name(&name).is_err());
    }

    #[test]
    fn recognizes_impl_future_return_types() {
        let qualified = syn::parse_str::<ReturnType>("-> impl ::std::future::Future<Output = u64>").unwrap();
        let imported = syn::parse_str::<ReturnType>("-> impl Future<Output = u64>").unwrap();
        assert!(returns_impl_future(&qualified));
        assert!(returns_impl_future(&imported));
    }
}
