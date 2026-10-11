//! The procedural macros of Itinera, which are syntax over its builder.
//!
//! Applications use them through the `macros` feature of the `itinera` crate, which re-exports
//! each from the module of its concept, such as `itinera::step::step`.

#![forbid(unsafe_code)]

mod block;
mod datum;
mod described;
mod marker;
mod policy;
mod step;

use proc_macro::TokenStream;
use syn::{DeriveInput, ItemImpl, parse_macro_input};

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
