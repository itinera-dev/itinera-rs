//! The procedural macros of Itinera, which are syntax over its builder.
//!
//! Applications use them through the `macros` feature of the `itinera` crate, which re-exports
//! each from the module of its concept, such as `itinera::step::step`.

#![forbid(unsafe_code)]

mod block;
mod datum;
mod described;
mod journey;
mod local;
mod marker;
mod policy;
mod role;
mod step;
mod workflow;

use proc_macro::TokenStream;
use syn::{DeriveInput, Item, ItemImpl, parse_macro_input};

#[doc = include_str!("../docs/step.md")]
#[proc_macro_attribute]
pub fn step(arguments: TokenStream, block: TokenStream) -> TokenStream {
    let block = parse_macro_input!(block as ItemImpl);
    step::expand(arguments.into(), block).into()
}

#[doc = include_str!("../docs/step_policy.md")]
#[proc_macro_attribute]
pub fn step_policy(arguments: TokenStream, block: TokenStream) -> TokenStream {
    let block = parse_macro_input!(block as ItemImpl);
    policy::expand(policy::Kind::Step, arguments.into(), block).into()
}

#[doc = include_str!("../docs/workflow_policy.md")]
#[proc_macro_attribute]
pub fn workflow_policy(arguments: TokenStream, block: TokenStream) -> TokenStream {
    let block = parse_macro_input!(block as ItemImpl);
    policy::expand(policy::Kind::Workflow, arguments.into(), block).into()
}

#[doc = include_str!("../docs/derive_step.md")]
#[proc_macro_derive(Step, attributes(input, default))]
pub fn derive_step(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    step::expand_derive(input).into()
}

#[doc = include_str!("../docs/derive_initial_data.md")]
#[proc_macro_derive(InitialData, attributes(key, required))]
pub fn derive_initial_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    journey::expand_initial_data(input).into()
}

#[doc = include_str!("../docs/derive_output.md")]
#[proc_macro_derive(Output, attributes(key, required))]
pub fn derive_output(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    journey::expand_output(input).into()
}

#[doc = include_str!("../docs/workflow.md")]
#[proc_macro_attribute]
pub fn workflow(arguments: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    workflow::expand(arguments.into(), item).into()
}

#[doc = include_str!("../docs/role.md")]
#[proc_macro_attribute]
pub fn role(arguments: TokenStream, block: TokenStream) -> TokenStream {
    let block = parse_macro_input!(block as ItemImpl);
    role::expand(arguments.into(), block).into()
}
