//! A service provider of the workflow's `impl` block: a method marked `#[provider]`, the service
//! it supplies, its scope, the steps it is attached to, and what it requests.

mod request;
mod settings;

use proc_macro2::TokenStream;
use quote::{ToTokens as _, format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Ident, ImplItemFn, LitStr, ReturnType, Type};

use super::{arguments, by_reference};
use crate::datum::held_by;
use crate::local::local;
use crate::{block, described};
pub(super) use request::MARKS;
use request::Parameter;
use settings::Settings;

/// A method marked as a service provider.
pub(super) struct Provider {
    name: LitStr,
    key: LitStr,
    per_attempt: bool,
    /// The steps it is attached to, none when it serves every step without a provider of its
    /// own for the service.
    steps: Vec<LitStr>,
    method: Ident,
    asynchronous: bool,
    parameters: Vec<Parameter>,
    /// The method's return type, as written.
    output: Type,
}

impl Provider {
    /// Reads the provider its mark describes, and removes its parameters' attributes.
    pub(super) fn read(function: &mut ImplItemFn, mark: &Attribute) -> syn::Result<Self> {
        if !cfg!(feature = "unstable") {
            return Err(syn::Error::new_spanned(
                mark,
                "a service provider needs the `unstable` feature of `itinera`",
            ));
        }
        let settings = Settings::read(arguments(mark, settings::MISREAD)?)?;
        if let Some(steps) = &settings.steps {
            if steps.is_empty() {
                return Err(syn::Error::new_spanned(
                    mark,
                    "a service provider attached to steps lists at least one, as in \
                     `steps = [\"charge\"]`: leave `steps` out for one attached to no step",
                ));
            }
        }
        if !function.sig.receiver().is_some_and(by_reference) {
            return Err(syn::Error::new_spanned(
                &function.sig,
                "a service provider takes the workflow as `&self`",
            ));
        }
        if let Some(asynchronous) = function.sig.asyncness {
            if !cfg!(feature = "async") {
                return Err(syn::Error::new_spanned(
                    asynchronous,
                    "an asynchronous service provider needs the `async` feature of `itinera`",
                ));
            }
        }
        let ReturnType::Type(_, output) = &function.sig.output else {
            return Err(syn::Error::new_spanned(
                &function.sig.ident,
                "a service provider returns the instance it makes, as `Result<Box<T>, Error>`",
            ));
        };
        let output = (**output).clone();
        let per_attempt = settings.per_attempt.is_some();
        let parameters = function
            .sig
            .inputs
            .iter_mut()
            .filter_map(block::typed)
            .map(|parameter| Parameter::read(parameter, per_attempt))
            .collect::<syn::Result<_>>()?;
        let method = function.sig.ident.clone();
        Ok(Self {
            name: settings
                .name
                .unwrap_or_else(|| described::named_after(&method)),
            key: settings
                .key
                .unwrap_or_else(|| described::named_after(&method)),
            per_attempt,
            steps: settings.steps.unwrap_or_default(),
            method,
            asynchronous: function.sig.asyncness.is_some(),
            parameters,
            output,
        })
    }

    pub(super) fn name(&self) -> &LitStr {
        &self.name
    }

    pub(super) fn key(&self) -> &LitStr {
        &self.key
    }

    pub(super) fn steps(&self) -> &[LitStr] {
        &self.steps
    }

    pub(super) fn is_asynchronous(&self) -> bool {
        self.asynchronous
    }

    pub(super) fn requests_a_reporter(&self) -> bool {
        self.parameters.iter().any(Parameter::is_reporter)
    }

    pub(super) fn shares_name_with(&self, other: &Self) -> bool {
        self.name.value() == other.name.value()
    }

    pub(super) fn shares_key_with(&self, other: &Self) -> bool {
        self.key.value() == other.key.value()
    }

    /// The type of the instances the provider makes, read from its return type when it is written
    /// `Result<Box<T>, ...>`.
    pub(super) fn made(&self) -> Option<&Type> {
        held_by(held_by(&self.output, "Result", 2)?, "Box", 1)
    }

    /// Whether both providers' return types are written as `Result<Box<T>, ...>` with the same
    /// `T`, written alike.
    pub(super) fn makes_as(&self, other: &Self) -> bool {
        match (self.made(), other.made()) {
            (Some(made), Some(other)) => spelled(made) == spelled(other),
            _ => false,
        }
    }

    /// The service the provider supplies and the steps it is attached to, as the checks of the
    /// workflow's struct read them.
    pub(super) fn listed(&self) -> TokenStream {
        let Self { key, steps, .. } = self;
        quote!((#key, &[#(#steps),*]))
    }

    /// The function the descriptor of a workflow in this mode calls, which resolves the requests
    /// and calls the method: an associated function of the workflow, so that the method's return
    /// type means the same there.
    pub(super) fn function(&self, workflow: &Type, asynchronous: bool) -> TokenStream {
        let function = self.function_name(asynchronous);
        let got = local("got");
        let workflow_value = local("workflow");
        let pattern = if self.parameters.is_empty() {
            quote!(_)
        } else if self.parameters.iter().any(Parameter::takes) {
            quote!(mut #got)
        } else {
            quote!(#got)
        };
        let arguments: Vec<Ident> = self
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| parameter.argument(index))
            .collect();
        let taken = self
            .parameters
            .iter()
            .map(|parameter| parameter.taken(&got));
        let output = &self.output;
        let span = output.span();
        let mut method = self.method.clone();
        method.set_span(span);
        let awaited = self.asynchronous.then_some(quote!(.await));
        let called =
            quote_spanned!(span=> <#workflow>::#method(#workflow_value, #(#arguments),*) #awaited);
        let scope = self.scope();
        let (asyncness, mode) = if asynchronous {
            (quote!(async), quote!(, ::itinera::mode::Asynchronous))
        } else {
            (quote!(), quote!())
        };
        quote! {
            #[doc(hidden)]
            #asyncness fn #function(
                #workflow_value: &#workflow,
                #pattern: ::itinera::policy::Requested<
                    '_,
                    #workflow,
                    ::itinera::policy::Provider<#scope>
                    #mode
                >,
            ) -> #output {
                #(let #arguments = #taken;)*
                #called
            }
        }
    }

    /// The provider's descriptor, for a workflow in this mode, attached to its steps, with what
    /// it needs.
    pub(super) fn descriptor(&self, workflow: &Type, asynchronous: bool) -> TokenStream {
        let Self { name, key, .. } = self;
        let function = self.function_name(asynchronous);
        let constructor = match (self.per_attempt, asynchronous) {
            (false, false) => quote!(new),
            (true, false) => quote!(per_attempt),
            (false, true) => quote!(new_async),
            (true, true) => quote!(per_attempt_async),
        };
        let needs = self.parameters.iter().filter_map(Parameter::need);
        let steps = &self.steps;
        let described = quote_spanned! {self.output.span()=>
            ::itinera::service::ServiceProviderDescriptor::#constructor(
                #name,
                &::itinera::service::Service::new(#key),
                <#workflow>::#function,
            )
        };
        quote! {
            #described
                .needing(::itinera::policy::HookNeeds::new() #(#needs)*)
                #(.step(::itinera::step::step_name!(#steps)))*
        }
    }

    fn scope(&self) -> TokenStream {
        if self.per_attempt {
            quote!(::itinera::service::PerAttempt)
        } else {
            quote!(::itinera::service::PerJourney)
        }
    }

    fn function_name(&self, asynchronous: bool) -> Ident {
        if asynchronous {
            format_ident!("__itinera_async_provider_{}", self.method)
        } else {
            format_ident!("__itinera_provider_{}", self.method)
        }
    }
}

/// The type as written, in tokens.
fn spelled(ty: &Type) -> String {
    ty.to_token_stream().to_string()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::super::fixtures::{provider, read};
    use super::*;

    /// The code without its spaces, which tokens place differently from the way code is written.
    fn compact(code: &str) -> String {
        code.split_whitespace().collect()
    }

    #[rstest]
    #[case::named_and_keyed_after_its_method(
        "#[provider] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        "stock",
        r#"("stock", &[])"#
    )]
    #[case::keyed_by_its_settings(
        r#"#[provider(key = "stock")] fn warehouse(&self) -> Result<Box<dyn StockStore>, Error> {}"#,
        "warehouse",
        r#"("stock", &[])"#
    )]
    #[case::named_by_its_settings(
        r#"#[provider(name = "warehouse")] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}"#,
        "warehouse",
        r#"("stock", &[])"#
    )]
    #[case::attached_to_steps(
        r#"#[provider(per_attempt, steps = ["charge", "refund"])] fn payments(&self) -> Result<Box<dyn PaymentGateway>, Error> {}"#,
        "payments",
        r#"("payments", &["charge", "refund"])"#
    )]
    fn a_provider_is_named_and_keyed_after_its_method_unless_its_settings_say_otherwise(
        #[case] method: &str,
        #[case] name: &str,
        #[case] listed: &str,
    ) {
        let provider = provider(method);
        assert_eq!(provider.name().value(), name);
        assert_eq!(compact(&provider.listed().to_string()), compact(listed));
    }

    #[rstest]
    #[case::attached_to_an_empty_list_of_steps(
        "#[provider(steps = [])] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        r#"a service provider attached to steps lists at least one, as in `steps = ["charge"]`: leave `steps` out for one attached to no step"#
    )]
    #[case::with_an_unknown_setting(
        "#[provider(per_journey)] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        r#"`#[provider]` takes, each optional, `per_attempt`, `steps = ["..."]`, `key = "..."` and `name = "..."`"#
    )]
    #[case::with_a_setting_given_twice(
        "#[provider(per_attempt, per_attempt)] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        "this setting is given once"
    )]
    #[case::taking_the_workflow_by_value(
        "#[provider] fn stock(self) -> Result<Box<dyn StockStore>, Error> {}",
        "a service provider takes the workflow as `&self`"
    )]
    #[case::without_a_return_type(
        "#[provider] fn stock(&self) {}",
        "a service provider returns the instance it makes, as `Result<Box<T>, Error>`"
    )]
    #[case::of_journey_scope_requesting_the_attempt(
        "#[provider] fn stock(&self, #[attempt] attempt: NonZeroU32) -> Result<Box<dyn StockStore>, Error> {}",
        "`#[attempt]` is requested only by a provider of attempt scope, marked `#[provider(per_attempt)]`: one of journey scope serves whichever step needs its service first"
    )]
    #[case::of_journey_scope_requesting_the_step_name(
        "#[provider] fn stock(&self, #[step_name] step: StepName) -> Result<Box<dyn StockStore>, Error> {}",
        "`#[step_name]` is requested only by a provider of attempt scope, marked `#[provider(per_attempt)]`: one of journey scope serves whichever step needs its service first"
    )]
    #[case::with_an_unmarked_parameter(
        "#[provider] fn stock(&self, gateway: String) -> Result<Box<dyn StockStore>, Error> {}",
        r#"every parameter of a service provider after `&self` says what it requests, such as `#[journey_id]` or `#[from_workflow("gateway")]`"#
    )]
    fn a_provider_is_refused_when_its_method_or_settings_cannot_describe_one(
        #[case] method: &str,
        #[case] refused: &str,
    ) {
        let error = read(method).err().map(|error| error.to_string());
        assert_eq!(error.as_deref(), Some(refused));
    }

    #[test]
    fn a_provider_requesting_what_its_scope_allows_declares_its_needs_in_order() {
        let provider = provider(
            "#[provider(per_attempt)] fn payments(&self, #[attempt] attempt: NonZeroU32, \
             #[from_workflow] gateway: Option<String>, #[reporter] reporter: HookReporter<'_>) \
             -> Result<Box<dyn PaymentGateway>, Error> {}",
        );
        let workflow: Type = syn::parse_str("Orders").unwrap();

        let descriptor = provider.descriptor(&workflow, false).to_string();

        assert!(compact(&descriptor).ends_with(&compact(
            r#".needing(::itinera::policy::HookNeeds::new()
                .optional_from_workflow(&::itinera::step::OptionalInput::<String>::new("gateway"))
                .reporter())"#
        )));
    }

    #[rstest]
    #[case::written_as_a_box_in_a_result(
        "#[provider] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        Some("dyn StockStore")
    )]
    #[case::written_with_paths(
        "#[provider] fn stock(&self) -> std::result::Result<std::boxed::Box<Shelf>, Error> {}",
        Some("Shelf")
    )]
    #[case::written_through_an_alias(
        "#[provider] fn stock(&self) -> Provided<dyn StockStore> {}",
        None
    )]
    fn a_provider_makes_the_type_its_return_type_boxes_in_a_result(
        #[case] method: &str,
        #[case] expected: Option<&str>,
    ) {
        let provider = provider(method);
        let made = provider.made().map(spelled);
        assert_eq!(made.as_deref(), expected);
    }

    #[rstest]
    #[case::boxing_one_type_with_errors_written_apart(
        "#[provider] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        "#[provider] fn shelf(&self) -> Result<Box<dyn StockStore>, itinera::error::Error> {}",
        true
    )]
    #[case::boxing_two_types(
        "#[provider] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}",
        "#[provider] fn shelf(&self) -> Result<Box<Shelf>, Error> {}",
        false
    )]
    #[case::one_returning_through_an_alias(
        "#[provider] fn stock(&self) -> Provided<dyn StockStore> {}",
        "#[provider] fn shelf(&self) -> Provided<dyn StockStore> {}",
        false
    )]
    fn two_providers_make_alike_only_when_both_box_one_type_written_alike(
        #[case] one: &str,
        #[case] other: &str,
        #[case] alike: bool,
    ) {
        assert_eq!(provider(one).makes_as(&provider(other)), alike);
    }
}
