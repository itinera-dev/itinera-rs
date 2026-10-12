//! A step the workflow lists, from its `#[with_step(...)]` line: its type, its name, its retry
//! budget, whether an abnormal termination is retriable, its policies, and its rescue step.

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{ParseStream, Parser as _};
use syn::spanned::Spanned as _;
use syn::{Attribute, Ident, LitInt, LitStr, Token, Type};

use super::checks::Attached;
use super::{key, list, once};
use crate::policy::Kind;

/// One step the workflow lists.
pub(super) struct WithStep {
    step: Type,
    pub(super) name: LitStr,
    retry_budget: Option<u16>,
    retriable: bool,
    policies: Vec<Type>,
    rescue: Option<Type>,
}

/// The settings that follow the step's type, each optional until checked.
#[derive(Default)]
struct Settings {
    name: Option<LitStr>,
    retry_budget: Option<u16>,
    retriable: Option<()>,
    policies: Option<Vec<Type>>,
    rescue: Option<Type>,
}

impl WithStep {
    /// Reads the step a `#[with_step(...)]` line lists.
    pub(super) fn read(mark: &Attribute) -> syn::Result<Self> {
        let (step, settings) = mark.parse_args_with(|input: ParseStream<'_>| {
            let step: Type = input.parse()?;
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
            let mut settings = Settings::default();
            syn::meta::parser(|meta| settings.read(&meta)).parse2(input.parse()?)?;
            Ok((step, settings))
        })?;
        let Some(name) = settings.name else {
            return Err(syn::Error::new_spanned(
                mark,
                "name the step, as in `#[with_step(Charge, name = \"charge\")]`: a step's name \
                 belongs to the workflow that lists it",
            ));
        };
        if let (Some(rescue), Some(1..)) = (&settings.rescue, settings.retry_budget) {
            return Err(syn::Error::new_spanned(
                rescue,
                "rescued step retried: a step with a rescue step is never retried, so it has no \
                 `retry_budget`",
            ));
        }
        Ok(Self {
            step,
            name,
            retry_budget: settings.retry_budget,
            retriable: settings.retriable.is_some(),
            policies: settings.policies.unwrap_or_default(),
            rescue: settings.rescue,
        })
    }

    /// The step's type.
    pub(super) fn step(&self) -> &Type {
        &self.step
    }

    /// Its rescue step's type, if it has one.
    pub(super) fn rescue(&self) -> Option<&Type> {
        self.rescue.as_ref()
    }

    pub(super) fn shares_name_with(&self, other: &Self) -> bool {
        self.name.value() == other.name.value()
    }

    pub(super) fn is_named_among(&self, names: &[String]) -> bool {
        names.contains(&self.name.value())
    }

    /// The name of its rescue step, if it has one: its own name followed by `_rescue`.
    pub(super) fn rescue_name(&self) -> Option<String> {
        self.rescue
            .is_some()
            .then(|| format!("{}_rescue", self.name.value()))
    }

    /// The builder's call that adds the step to a workflow of this type.
    pub(super) fn declared(&self, workflow: &Ident) -> TokenStream {
        let Self { step, name, .. } = self;
        let retry_budget = self.retry_budget.map(retry_budget);
        let retriable = self.retriable.then(retriable);
        let policies = self
            .policies
            .iter()
            .map(|policy| attached(policy, workflow));
        let rescue = self.rescue.as_ref().map(rescued_by);
        let described = quote_spanned! {step.span()=>
            <#step as ::itinera::step::DescribedStep>::descriptor::<#workflow>(
                ::itinera::step::step_name!(#name)
            )
        };
        quote!(.step(#described #retry_budget #retriable #(#policies)* #rescue))
    }

    /// The checks, run as the program compiles, that no two of the step's policies define the
    /// same hook.
    pub(super) fn checked(&self, workflow: &Ident) -> Option<TokenStream> {
        Attached {
            kind: Kind::Step,
            to: format!("step {}", self.name.value()),
            workflow,
            policies: &self.policies,
        }
        .checked()
    }
}

impl Settings {
    fn read(&mut self, meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
        match key(meta).as_str() {
            "name" => once(&mut self.name, meta.value()?.parse()?, meta),
            "retry_budget" => once(&mut self.retry_budget, budget(meta)?, meta),
            "abnormal_termination_retriable" => once(&mut self.retriable, (), meta),
            "policies" => once(&mut self.policies, policies(meta)?, meta),
            "rescue" => once(&mut self.rescue, meta.value()?.parse()?, meta),
            _ => Err(meta.error(
                "`#[with_step]` takes the step's type, then `name = \"...\"`, and optionally \
                 `retry_budget = N`, `abnormal_termination_retriable`, `policies = [...]` and \
                 `rescue = RescueStep`",
            )),
        }
    }
}

fn budget(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<u16> {
    let budget: LitInt = meta.value()?.parse()?;
    budget.base10_parse().map_err(|_| {
        syn::Error::new_spanned(
            &budget,
            "a retry budget is a number of retries, from 0 to 65535",
        )
    })
}

fn policies(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<Vec<Type>> {
    Ok(list(meta)?.into_iter().collect())
}

fn retry_budget(budget: u16) -> TokenStream {
    quote!(.retry_budget(#budget))
}

fn retriable() -> TokenStream {
    quote!(.abnormal_termination_retriable())
}

/// The builder's call that gives the step its rescue step.
fn rescued_by(rescue: &Type) -> TokenStream {
    quote_spanned! {rescue.span()=>
        .rescued_by::<#rescue>()
    }
}

/// The builder's call that attaches the policy to the step.
fn attached(policy: &Type, workflow: &Ident) -> TokenStream {
    quote_spanned! {policy.span()=>
        .policy(<#policy as ::itinera::policy::StepPolicy<#workflow>>::descriptor())
    }
}
