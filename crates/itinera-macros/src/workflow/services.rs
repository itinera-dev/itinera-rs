//! The checks, run as the program compiles, that the workflow provides each service its steps
//! and rescue steps request: a provider of its key attached to the step, or one attached to no
//! step, which alone serves a rescue step; and that its providers of that key make the type the
//! step requests.
//!
//! The services are read from what the steps' `DescribedStep` and the rescue steps'
//! `DescribedRescueStep` list, and the providers from what the workflow's `impl` block declares:
//! none when it has no block or no provider, and not known when the block could not be read,
//! which an error already reports.

mod functions;

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Ident, LitStr, Type};

use super::with_step::WithStep;

/// The checks for every step of a workflow of this type.
pub(super) fn checked(workflow: &Ident, steps: &[WithStep]) -> TokenStream {
    let checks = steps.iter().map(|step| provided(workflow, step));
    let fits = steps.iter().map(|step| fit(workflow, step));
    let functions = functions::functions();
    quote! {
        trait __ItineraServicesFit {
            fn __itinera_services_fit<__ItineraWorkflow>() {}
        }

        impl<S: ?Sized> __ItineraServicesFit for S {}

        #(#fits)*

        trait __ItineraProvided {
            const __ITINERA_PROVIDERS: ::core::option::Option<
                &'static [(&'static str, &'static [&'static str])],
            > = ::core::option::Option::Some(&[]);
        }

        impl __ItineraProvided for #workflow {}

        #functions

        #(#checks)*
    }
}

/// The checks that the workflow provides each service the step and its rescue step request.
fn provided(workflow: &Ident, step: &WithStep) -> TokenStream {
    let name = &step.name;
    let ty = step.step();
    let services = quote!(<#ty as ::itinera::step::DescribedStep>::SERVICES);
    let checked = unprovided(workflow, name.span(), &quote!(Some(#name)), name, &services);
    let rescue = step
        .rescue()
        .zip(step.rescue_name())
        .map(|(rescue, name)| rescue_unprovided(workflow, rescue, &name));
    quote!(#checked #rescue)
}

/// The check that the workflow provides each service the rescue step of this name requests,
/// reported at its type.
fn rescue_unprovided(workflow: &Ident, rescue: &Type, name: &str) -> TokenStream {
    let services = quote!(<#rescue as ::itinera::step::DescribedRescueStep>::SERVICES);
    let name = LitStr::new(name, rescue.span());
    unprovided(workflow, rescue.span(), &quote!(None), &name, &services)
}

/// The check that the workflow provides each of these services to the step it serves as
/// `served`, reported where `span` is.
fn unprovided(
    workflow: &Ident,
    span: Span,
    served: &TokenStream,
    name: &LitStr,
    services: &TokenStream,
) -> TokenStream {
    quote_spanned! {span=>
        const _: () = if let ::core::option::Option::Some(key) = __itinera_services::unprovided(
            <#workflow>::__ITINERA_PROVIDERS,
            ::core::option::Option::#served,
            #services,
        ) {
            ::core::panic!(
                "{}",
                __itinera_services::text(&__itinera_services::message(&[
                    "service not provided: ",
                    key,
                    " for ",
                    #name,
                ]))
            );
        };
    }
}

/// The checks that the workflow's providers make each service the step and its rescue step
/// request as the type they request, reported at the step's name and at the rescue step's type.
fn fit(workflow: &Ident, step: &WithStep) -> TokenStream {
    let checked = fitting(workflow, step.step(), step.name.span());
    let rescue = step
        .rescue()
        .map(|rescue| fitting(workflow, rescue, rescue.span()));
    quote!(#checked #rescue)
}

/// The check that the workflow's providers make each service the step requests as the type it
/// requests, reported where `span` is: the bounds of the step's hidden function, which a step
/// the macros did not describe has none of.
fn fitting(workflow: &Ident, ty: &Type, span: Span) -> TokenStream {
    let workflow = Ident::new(&workflow.to_string(), span);
    quote_spanned! {span=>
        const _: () = {
            let _ = <#ty>::__itinera_services_fit::<#workflow>;
        };
    }
}
