//! What `#[step_policy]` and `#[workflow_policy]` implement when they refuse a block, so that the
//! compiler reports their error alone.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemImpl;

use super::Kind;
use crate::described;

/// A `DescribedPolicy` and a policy trait that are never called, in the mode the block's hooks
/// are marked with.
pub(super) fn stand_in(kind: Kind, block: &ItemImpl, asynchronous: bool) -> TokenStream {
    let (generics, _, where_clause) = block.generics.split_for_impl();
    let policy = &block.self_ty;
    let (policy_trait, descriptor, hook_type) = kind.names();
    let mode = described::mode(asynchronous);
    let workflow = super::workflow();
    quote! {
        impl #generics ::itinera::policy::DescribedPolicy for #policy #where_clause {
            type Mode = #mode;

            fn name() -> ::itinera::policy::PolicyName {
                ::core::unreachable!()
            }

            fn build() -> ::core::result::Result<Self, ::itinera::error::Error> {
                ::core::unreachable!()
            }
        }

        impl<#workflow> ::itinera::policy::#policy_trait<#workflow> for #policy {
            const HOOKS: &'static [::itinera::policy::#hook_type] = &[];

            fn descriptor() -> ::itinera::policy::#descriptor<Self, #workflow, #mode> {
                ::core::unreachable!()
            }
        }
    }
}
