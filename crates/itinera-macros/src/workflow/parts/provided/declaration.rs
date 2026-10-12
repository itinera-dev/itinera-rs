//! The providers' declaration on the workflow, in the mode of the workflow's struct, which their
//! block cannot see.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{ItemImpl, Type};

use crate::described;
use crate::service_key::key_type;
use crate::workflow::parts::provider::Provider;

/// The providers' declaration on the workflow: the functions their descriptors call, the trait
/// through which a workflow of either mode declares them, its implementation for each mode they
/// can belong to, what they provide, which the checks of the workflow's struct read, and what
/// they make of each key, which the bounds of its steps read.
///
/// An asynchronous provider belongs only to an asynchronous workflow, so the trait is implemented
/// for a synchronous one only when every provider is synchronous. A synchronous provider belongs
/// to workflows of both modes, unless it requests a reporter, which only a synchronous workflow
/// gives it, since it could not await an asynchronous one.
pub(in crate::workflow::parts) fn declared(
    workflow: &Type,
    providers: &[&Provider],
) -> TokenStream {
    let synchronous =
        (!providers.iter().any(is_asynchronous)).then(|| declared_in(workflow, providers, false));
    let asynchronous = (cfg!(feature = "async") && !providers.iter().any(reports_synchronously))
        .then(|| declared_in(workflow, providers, true));
    let listed = providers.iter().map(|provider| provider.listed());
    let made = providers
        .iter()
        .enumerate()
        .filter_map(|(index, provider)| first_of_its_key(providers, index, provider))
        .map(|provider| made_of_key(workflow, providers, provider));
    quote! {
        impl #workflow {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub(crate) const __ITINERA_PROVIDERS: ::core::option::Option<
                &'static [(&'static str, &'static [&'static str])],
            > = ::core::option::Option::Some(&[#(#listed),*]);
        }

        #[diagnostic::on_unimplemented(
            message = "the service providers of `{Self}` do not all belong to a workflow of its \
                       mode",
            label = "a workflow of a mode some of its providers do not belong to",
            note = "an asynchronous provider belongs only to an asynchronous workflow, and a \
                    synchronous one that requests a reporter only to a synchronous workflow"
        )]
        pub trait __ItineraProviders<__ItineraMode: ::itinera::mode::Mode> {
            fn __itinera_providers(
                declared: ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>,
            ) -> ::itinera::workflow::WorkflowBuilder<#workflow, __ItineraMode>;
        }

        #synchronous
        #asynchronous
        #(#made)*
    }
}

/// What a block that could not be read provides: not known, so that the checks of the workflow's
/// struct take every service as provided, of whatever type. A block that cannot name the
/// workflow says nothing.
pub(in crate::workflow::parts) fn unknown(block: &ItemImpl) -> Option<TokenStream> {
    if !cfg!(feature = "unstable") || block.trait_.is_some() || !block.generics.params.is_empty() {
        return None;
    }
    let workflow = &block.self_ty;
    Some(quote! {
        const _: () = {
            impl #workflow {
                #[doc(hidden)]
                #[allow(dead_code)]
                pub(crate) const __ITINERA_PROVIDERS: ::core::option::Option<
                    &'static [(&'static str, &'static [&'static str])],
                > = ::core::option::Option::None;
            }

            impl<K, T: ?Sized> ::itinera::service::ProvidesService<K, T> for #workflow {}
        };
    })
}

/// The provider, if no earlier one supplies its key.
fn first_of_its_key<'a>(
    providers: &[&Provider],
    index: usize,
    provider: &'a Provider,
) -> Option<&'a Provider> {
    let earlier = providers
        .iter()
        .take(index)
        .any(|earlier| earlier.shares_key_with(provider));
    (!earlier).then_some(provider)
}

/// What the providers of this provider's key make: exactly the type their return types box, when
/// every one of them writes its return type as `Result<Box<T>, ...>` with the same `T`, and
/// otherwise any type, which leaves the steps they serve to `build()`.
fn made_of_key(workflow: &Type, providers: &[&Provider], first: &Provider) -> TokenStream {
    let key = key_type(first.key());
    let alike = providers
        .iter()
        .filter(|provider| provider.shares_key_with(first))
        .all(|provider| provider.makes_as(first));
    match first.made() {
        Some(made) if alike => quote! {
            impl ::itinera::service::ProvidesService<#key, #made> for #workflow {}
        },
        _ => quote! {
            impl<T: ?Sized> ::itinera::service::ProvidesService<#key, T> for #workflow {}
        },
    }
}

fn is_asynchronous(provider: &&Provider) -> bool {
    provider.is_asynchronous()
}

fn reports_synchronously(provider: &&Provider) -> bool {
    !provider.is_asynchronous() && provider.requests_a_reporter()
}

/// The trait's implementation for a workflow of one mode, and the functions it calls.
fn declared_in(workflow: &Type, providers: &[&Provider], asynchronous: bool) -> TokenStream {
    let functions = providers
        .iter()
        .map(|provider| provider.function(workflow, asynchronous));
    let descriptors = providers
        .iter()
        .map(|provider| provider.descriptor(workflow, asynchronous));
    let mode = described::mode(asynchronous);
    quote! {
        impl #workflow {
            #(#functions)*
        }

        impl __ItineraProviders<#mode> for #workflow {
            fn __itinera_providers(
                declared: ::itinera::workflow::WorkflowBuilder<#workflow, #mode>,
            ) -> ::itinera::workflow::WorkflowBuilder<#workflow, #mode> {
                declared #(.provider(#descriptors))*
            }
        }
    }
}
