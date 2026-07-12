use std::collections::HashSet;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    Expr, ExprAssign, ExprField, ExprPath, GenericArgument, ItemFn, LitStr, Member, Path, PathArguments, ReturnType,
    Token, Type, TypeParamBound,
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
    if key == "le" || key == "quantile" || key == "__name__" || key.starts_with("__") {
        return Err(syn::Error::new_spanned(
            expression,
            format!("label name `{key}` is reserved for Prometheus or function-metrics"),
        ));
    }
    Ok(key)
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Instrument {
    Duration,
    Calls,
    Errors,
}

impl Instrument {
    fn parse(identifier: &syn::Ident) -> syn::Result<Self> {
        match identifier.to_string().as_str() {
            "duration" => Ok(Self::Duration),
            "calls" => Ok(Self::Calls),
            "errors" => Ok(Self::Errors),
            _ => Err(syn::Error::new(
                identifier.span(),
                "expected `duration`, `calls`, or `errors`",
            )),
        }
    }
}

#[derive(Default)]
struct FunctionMetricsArgs {
    name: Option<LitStr>,
    labels: Vec<Label>,
    instruments: Option<HashSet<Instrument>>,
    error_classifier: Option<Path>,
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
                "metrics" => {
                    if args.instruments.is_some() {
                        return Err(syn::Error::new(option.span(), "duplicate `metrics` option"));
                    }
                    let content;
                    syn::parenthesized!(content in input);
                    let identifiers = Punctuated::<syn::Ident, Token![,]>::parse_terminated(&content)?;
                    if identifiers.is_empty() {
                        return Err(syn::Error::new(option.span(), "`metrics(...)` must not be empty"));
                    }
                    let mut instruments = HashSet::new();
                    for identifier in identifiers {
                        let instrument = Instrument::parse(&identifier)?;
                        if !instruments.insert(instrument) {
                            return Err(syn::Error::new(
                                identifier.span(),
                                format!("duplicate metric `{}`", identifier),
                            ));
                        }
                    }
                    args.instruments = Some(instruments);
                }
                "error_classifier" => {
                    if args.error_classifier.is_some() {
                        return Err(syn::Error::new(option.span(), "duplicate `error_classifier` option"));
                    }
                    input.parse::<Token![=]>()?;
                    args.error_classifier = Some(input.parse()?);
                }
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "expected `name = \"operation\"`, `metrics(...)`, `error_classifier = path`, or `labels(...)`",
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

/// Instruments a function with Prometheus-compatible function metrics.
///
/// `metrics(duration)` emits `{operation}_duration_seconds`, `metrics(calls)`
/// emits `{operation}_calls_total`, and `metrics(errors)` emits
/// `{operation}_errors_total`. Omitting `metrics(...)` enables duration only.
/// Errors are detected from a visible `Result` return type or by an
/// `error_classifier = path` function accepting a reference to the return
/// value and returning `bool`.
///
/// Duration and calls record on normal returns, panics, and cancellation after
/// polling begins. Errors count returned or classified errors only.
/// `#[track_caller]` functions and non-async functions returning `impl Future`
/// are rejected because they cannot be instrumented without changing semantics.
/// Renamed Future imports and concrete future return-type aliases cannot be detected.
///
/// ```ignore
/// #[function_metrics]
/// async fn refresh_cache() {}
///
/// #[function_metrics(
///     name = "handle_request",
///     metrics(duration, calls, errors),
///     labels(method, service = "api")
/// )]
/// async fn handle_request(method: Method) -> Result<(), Error> { Ok(()) }
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

    let instruments = args
        .instruments
        .unwrap_or_else(|| HashSet::from([Instrument::Duration]));
    let errors_enabled = instruments.contains(&Instrument::Errors);
    if let Some(classifier) = &args.error_classifier {
        if !errors_enabled {
            return syn::Error::new_spanned(classifier, "`error_classifier` requires `metrics(..., errors)`")
                .into_compile_error()
                .into();
        }
    }
    if errors_enabled && args.error_classifier.is_none() && !returns_result(&input_fn.sig.output) {
        return syn::Error::new_spanned(
            &input_fn.sig.output,
            "the `errors` metric requires a visible `Result` return type or `error_classifier = path`",
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
    let duration_name = instruments.contains(&Instrument::Duration).then(|| {
        LitStr::new(
            &format!("{}_duration_seconds", operation_name.value()),
            operation_name.span(),
        )
    });
    let calls_name = instruments.contains(&Instrument::Calls).then(|| {
        LitStr::new(
            &format!("{}_calls_total", operation_name.value()),
            operation_name.span(),
        )
    });
    let errors_name = errors_enabled.then(|| {
        LitStr::new(
            &format!("{}_errors_total", operation_name.value()),
            operation_name.span(),
        )
    });
    let duration_description = LitStr::new(
        &format!("Duration of `{}` function executions.", operation_name.value()),
        operation_name.span(),
    );
    let calls_description = LitStr::new(
        &format!("Number of `{}` function executions.", operation_name.value()),
        operation_name.span(),
    );
    let errors_description = LitStr::new(
        &format!(
            "Number of `{}` function executions that returned an error.",
            operation_name.value()
        ),
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

    let duration = descriptor_tokens(duration_name.as_ref(), &duration_description, &runtime);
    let calls = descriptor_tokens(calls_name.as_ref(), &calls_description, &runtime);
    let errors = descriptor_tokens(errors_name.as_ref(), &errors_description, &runtime);
    let guard_ident = syn::Ident::new("__function_metrics_guard", Span::mixed_site());
    let result_ident = syn::Ident::new("__function_metrics_result", Span::mixed_site());
    let start_guard = quote! {
        let #guard_ident = #runtime::__private::InvocationGuard::start(
            #duration,
            #calls,
            #errors,
            ::std::vec![#(#labels),*],
        );
    };

    let classify_error = if let Some(classifier) = args.error_classifier {
        quote! { #classifier(&#result_ident) }
    } else if errors_enabled {
        quote! { ::std::result::Result::is_err(&#result_ident) }
    } else {
        quote! { false }
    };
    let original_block = &input_fn.block;

    let is_native_async = input_fn.sig.asyncness.is_some();
    let is_async_trait_future = !is_native_async && returns_future(&input_fn.sig.output);

    let new_block = if is_native_async && errors_enabled {
        quote! {{
            #start_guard
            let #result_ident = (async #original_block).await;
            #guard_ident.complete(#classify_error);
            #result_ident
        }}
    } else if is_native_async {
        quote! {{
            #start_guard
            #original_block
        }}
    } else if is_async_trait_future {
        quote! {{
            ::std::boxed::Box::pin(async move {
                #start_guard
                let #result_ident = (|| #original_block)().await;
                #guard_ident.complete(#classify_error);
                #result_ident
            })
        }}
    } else if errors_enabled {
        quote! {{
            #start_guard
            let #result_ident = (|| #original_block)();
            #guard_ident.complete(#classify_error);
            #result_ident
        }}
    } else {
        quote! {{
            #start_guard
            #original_block
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

fn descriptor_tokens(name: Option<&LitStr>, description: &LitStr, runtime: &TokenStream2) -> TokenStream2 {
    match name {
        Some(name) => quote! {
            ::std::option::Option::Some(
                #runtime::__private::MetricDescriptor::new(#name, #description)
            )
        },
        None => quote! { ::std::option::Option::None },
    }
}

fn validate_operation_name(name: &LitStr) -> syn::Result<()> {
    let value = name.value();
    validate_snake_case_name(&value, name, "operation")?;
    if ["_duration_seconds", "_calls_total", "_errors_total"]
        .iter()
        .any(|suffix| value.ends_with(suffix))
    {
        return Err(syn::Error::new_spanned(
            name,
            "operation names must not include a generated metric suffix",
        ));
    }
    Ok(())
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
    boxed_future_trait(return_type).is_some()
}

fn returns_result(return_type: &ReturnType) -> bool {
    let ReturnType::Type(_, return_type) = return_type else {
        return false;
    };
    if is_result_type(return_type) {
        return true;
    }

    let Some(trait_bound) = boxed_future_trait(return_type) else {
        return false;
    };

    let Some(future) = trait_bound.path.segments.last() else {
        return false;
    };
    let PathArguments::AngleBracketed(arguments) = &future.arguments else {
        return false;
    };
    arguments.args.iter().any(|argument| {
        matches!(argument, GenericArgument::AssocType(output) if output.ident == "Output" && is_result_type(&output.ty))
    })
}

fn boxed_future_trait(return_type: &Type) -> Option<&syn::TraitBound> {
    let pin_inner = single_type_argument(return_type, "Pin")?;
    let box_inner = single_type_argument(pin_inner, "Box")?;
    let Type::TraitObject(trait_object) = box_inner else {
        return None;
    };

    trait_object.bounds.iter().find_map(|bound| match bound {
        TypeParamBound::Trait(trait_bound) if is_standard_future_path(&trait_bound.path) => Some(trait_bound),
        _ => None,
    })
}

fn is_result_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "Result"))
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
