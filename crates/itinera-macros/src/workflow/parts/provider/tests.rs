use rstest::rstest;

use super::super::fixtures::{provider, read};
use super::*;

/// The code without its spaces, which tokens place differently from the way code is written.
fn compact(code: &str) -> String {
    code.split_whitespace().collect()
}

#[rstest]
#[case::named_and_keyed_after_its_method(
    "#[provider] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    "stock",
    r#"("stock", &[])"#
)]
#[case::keyed_by_its_settings(
    r#"#[provider(key = "stock")] fn warehouse(&self) -> Result<Provided<dyn StockStore>, Error> {}"#,
    "warehouse",
    r#"("stock", &[])"#
)]
#[case::named_by_its_settings(
    r#"#[provider(name = "warehouse")] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}"#,
    "warehouse",
    r#"("stock", &[])"#
)]
#[case::attached_to_steps(
    r#"#[provider(per_attempt, steps = ["charge", "refund"])] fn payments(&self) -> Result<Provided<dyn PaymentGateway>, Error> {}"#,
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
    "#[provider(steps = [])] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    r#"a service provider attached to steps lists at least one, as in `steps = ["charge"]`: leave `steps` out for one attached to no step"#
)]
#[case::with_an_unknown_setting(
    "#[provider(per_journey)] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    r#"`#[provider]` takes, each optional, `per_attempt`, `steps = ["..."]`, `key = "..."` and `name = "..."`"#
)]
#[case::with_a_setting_given_twice(
    "#[provider(per_attempt, per_attempt)] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    "this setting is given once"
)]
#[case::taking_the_workflow_by_value(
    "#[provider] fn stock(self) -> Result<Provided<dyn StockStore>, Error> {}",
    "a service provider takes the workflow as `&self`"
)]
#[case::without_a_return_type(
    "#[provider] fn stock(&self) {}",
    "a service provider returns the instance it makes, as `Result<Provided<T>, Error>`"
)]
#[case::of_journey_scope_requesting_the_attempt(
    "#[provider] fn stock(&self, #[attempt] attempt: NonZeroU32) -> Result<Provided<dyn StockStore>, Error> {}",
    "`#[attempt]` is requested only by a provider of attempt scope, marked `#[provider(per_attempt)]`: one of journey scope serves whichever step needs its service first"
)]
#[case::of_journey_scope_requesting_the_step_name(
    "#[provider] fn stock(&self, #[step_name] step: StepName) -> Result<Provided<dyn StockStore>, Error> {}",
    "`#[step_name]` is requested only by a provider of attempt scope, marked `#[provider(per_attempt)]`: one of journey scope serves whichever step needs its service first"
)]
#[case::with_an_unmarked_parameter(
    "#[provider] fn stock(&self, gateway: String) -> Result<Provided<dyn StockStore>, Error> {}",
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
         -> Result<Provided<dyn PaymentGateway>, Error> {}",
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
#[case::written_as_provided_in_a_result(
    "#[provider] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    Some("dyn StockStore")
)]
#[case::written_as_provided_in_the_result_of_itinera(
    "#[provider] fn stock(&self) -> Result<Provided<dyn StockStore>> {}",
    Some("dyn StockStore")
)]
#[case::written_with_paths(
    "#[provider] fn stock(&self) -> std::result::Result<itinera::service::Provided<Shelf>, Error> {}",
    Some("Shelf")
)]
#[case::written_through_an_alias(
    "#[provider] fn stock(&self) -> Provided<dyn StockStore> {}",
    None
)]
fn a_provider_makes_the_type_its_return_type_provides_in_a_result(
    #[case] method: &str,
    #[case] expected: Option<&str>,
) {
    let provider = provider(method);
    let made = provider.made().map(spelled);
    assert_eq!(made.as_deref(), expected);
}

#[rstest]
#[case::providing_one_type_with_errors_written_apart(
    "#[provider] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    "#[provider] fn shelf(&self) -> Result<Provided<dyn StockStore>, itinera::error::Error> {}",
    true
)]
#[case::providing_one_type_with_and_without_the_error(
    "#[provider] fn stock(&self) -> Result<Provided<dyn StockStore>> {}",
    "#[provider] fn shelf(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    true
)]
#[case::providing_two_types(
    "#[provider] fn stock(&self) -> Result<Provided<dyn StockStore>, Error> {}",
    "#[provider] fn shelf(&self) -> Result<Provided<Shelf>, Error> {}",
    false
)]
#[case::one_returning_through_an_alias(
    "#[provider] fn stock(&self) -> Provided<dyn StockStore> {}",
    "#[provider] fn shelf(&self) -> Provided<dyn StockStore> {}",
    false
)]
fn two_providers_make_alike_only_when_both_provide_one_type_written_alike(
    #[case] one: &str,
    #[case] other: &str,
    #[case] alike: bool,
) {
    assert_eq!(provider(one).makes_as(&provider(other)), alike);
}
