//! The workflow's service providers together: each named once, each service provided once to no
//! step and once to each step, attached only to steps the workflow lists or their rescue steps,
//! and declared on the workflow in its mode.

mod declaration;

use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{LitStr, Type};

use super::provider::Provider;
use crate::workflow::repeated;
pub(super) use declaration::{declared, unknown};

/// Refuses a provider named like another.
pub(super) fn refuse_names_given_twice(providers: &[&Provider]) -> syn::Result<()> {
    match repeated(providers, share_a_name) {
        Some(provider) => Err(syn::Error::new_spanned(
            provider.name(),
            format!(
                "duplicate provider name: {}, which another service provider of the workflow has",
                provider.name().value()
            ),
        )),
        None => Ok(()),
    }
}

fn share_a_name(one: &&Provider, other: &&Provider) -> bool {
    one.shares_name_with(other)
}

/// Refuses a provider that lists a step twice.
pub(super) fn refuse_steps_listed_twice(providers: &[&Provider]) -> syn::Result<()> {
    match providers.iter().find_map(step_listed_twice) {
        Some(step) => Err(syn::Error::new_spanned(
            step,
            format!("this service provider lists step {} already", step.value()),
        )),
        None => Ok(()),
    }
}

fn step_listed_twice<'a>(provider: &&'a Provider) -> Option<&'a LitStr> {
    repeated(provider.steps(), same_step)
}

fn same_step(one: &LitStr, other: &LitStr) -> bool {
    one.value() == other.value()
}

/// Refuses two providers of one service attached to no step, or to the same step.
pub(super) fn refuse_services_provided_twice(providers: &[&Provider]) -> syn::Result<()> {
    let twice = providers
        .iter()
        .enumerate()
        .find_map(|(index, provider)| provided_before(providers, index, provider));
    match twice {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// The error for the provider at this index, when an earlier provider of its service is attached
/// to no step as it is, or to one of its steps.
fn provided_before(
    providers: &[&Provider],
    index: usize,
    provider: &Provider,
) -> Option<syn::Error> {
    providers
        .iter()
        .take(index)
        .filter(|earlier| earlier.shares_key_with(provider))
        .find_map(|earlier| overlap(earlier, provider))
}

/// The error for two providers of one service, when they serve the same steps.
fn overlap(earlier: &Provider, provider: &Provider) -> Option<syn::Error> {
    let key = provider.key().value();
    if earlier.steps().is_empty() && provider.steps().is_empty() {
        return Some(syn::Error::new_spanned(
            provider.key(),
            format!("service provided twice: {key}, by providers attached to no step"),
        ));
    }
    let step = provider.steps().iter().find(|step| lists(earlier, step))?;
    Some(syn::Error::new_spanned(
        step,
        format!("service provided twice: {key} for {}", step.value()),
    ))
}

fn lists(provider: &Provider, step: &LitStr) -> bool {
    provider
        .steps()
        .iter()
        .any(|listed| same_step(listed, step))
}

/// The checks, run as the program compiles, that each step the provider is attached to is one
/// the workflow lists or the rescue step of one.
pub(super) fn known<'a>(
    provider: &'a Provider,
    workflow: &'a Type,
) -> impl Iterator<Item = TokenStream> + 'a {
    provider
        .steps()
        .iter()
        .map(move |step| listed(provider, step, workflow))
}

fn listed(provider: &Provider, step: &LitStr, workflow: &Type) -> TokenStream {
    let message = format!(
        "service provider for unknown step: {} on {}",
        provider.name().value(),
        step.value()
    );
    quote_spanned! {step.span()=>
        const _: () = ::core::assert!(
            __itinera_steps::listed(<#workflow>::__ITINERA_STEPS, #step),
            #message
        );
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::super::fixtures::provider;
    use super::*;

    fn providers(methods: &[&str]) -> Vec<Provider> {
        methods.iter().copied().map(provider).collect()
    }

    const STOCK: &str = "#[provider] fn stock(&self) -> Result<Box<dyn StockStore>, Error> {}";
    const WAREHOUSE: &str = r#"#[provider(key = "stock")] fn warehouse(&self) -> Result<Box<dyn StockStore>, Error> {}"#;
    const SHIPPING_STOCK: &str = r#"#[provider(key = "stock", steps = ["ship"])] fn shipping(&self) -> Result<Box<dyn StockStore>, Error> {}"#;
    const PACKING_STOCK: &str = r#"#[provider(key = "stock", steps = ["pack", "ship"])] fn packing(&self) -> Result<Box<dyn StockStore>, Error> {}"#;
    const PAYMENTS: &str = r#"#[provider(steps = ["ship"])] fn payments(&self) -> Result<Box<dyn PaymentGateway>, Error> {}"#;

    #[rstest]
    #[case::two_attached_to_no_step(&[STOCK, WAREHOUSE], Some("service provided twice: stock, by providers attached to no step"))]
    #[case::two_attached_to_one_step(&[SHIPPING_STOCK, PACKING_STOCK], Some("service provided twice: stock for ship"))]
    #[case::one_attached_to_no_step_and_one_to_a_step(&[STOCK, SHIPPING_STOCK], None)]
    #[case::two_of_different_services_attached_to_one_step(&[SHIPPING_STOCK, PAYMENTS], None)]
    fn a_service_is_provided_once_to_no_step_and_once_to_each_step(
        #[case] methods: &[&str],
        #[case] refused: Option<&str>,
    ) {
        let providers = providers(methods);
        let providers: Vec<&Provider> = providers.iter().collect();

        let error = refuse_services_provided_twice(&providers).err();

        assert_eq!(error.map(|error| error.to_string()).as_deref(), refused);
    }

    #[test]
    fn two_providers_of_one_name_are_refused() {
        let named = r#"#[provider(name = "stock", key = "warehouse")] fn warehouse(&self) -> Result<Box<dyn StockStore>, Error> {}"#;
        let providers = providers(&[STOCK, named]);
        let providers: Vec<&Provider> = providers.iter().collect();

        let error = refuse_names_given_twice(&providers).err();

        assert_eq!(
            error.map(|error| error.to_string()).as_deref(),
            Some(
                "duplicate provider name: stock, which another service provider of the workflow has"
            )
        );
    }
}
