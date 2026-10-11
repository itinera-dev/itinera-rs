//! The instance builder `#[workflow]` gives the workflow: one setter per field of its struct,
//! then its initial data and reporters, and `create()`, which compiles only once every required
//! field is set.

mod field;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Visibility};

pub(super) use field::{Settable, settable};

use crate::local::local;

/// `Workflow::builder()`, the builder's type, and the trait through which `create()` takes each
/// field, for a workflow of this type and mode with these fields. When some of its fields could
/// not be read, `create()` is a stand-in that is never called.
pub(super) fn builder(
    workflow: &Ident,
    visibility: &Visibility,
    fields: &[Settable],
    mode: &TokenStream,
    asynchronous: bool,
    complete: bool,
) -> TokenStream {
    let instance_builder = quote!(::itinera::instance::InstanceBuilder<#workflow, #mode>);
    let states: Vec<Ident> = (0..fields.len()).map(state).collect();
    let unset = vec![format_ident!("__ItineraUnset"); fields.len()];
    let traits = fields
        .iter()
        .enumerate()
        .map(|(index, field)| field.taken(workflow, index));
    let setters = fields
        .iter()
        .enumerate()
        .map(|(index, field)| field.setter(index, &states));
    let bounds = (0..fields.len()).map(bound);
    let assigned = fields
        .iter()
        .enumerate()
        .map(|(index, field)| field.assigned(index));
    let variables: Vec<Ident> = (0..fields.len()).map(variable).collect();
    let workflow_value = local("workflow");
    let instance = local("instance");
    let call = local("call");
    let created = if complete {
        quote! {
            let (#(#variables,)*) = self.fields;
            let #workflow_value = #workflow { #(#assigned),* };
            let #instance = <#workflow as ::itinera::workflow::Workflow>::descriptor()
                .instance(#workflow_value);
            self.calls
                .into_iter()
                .fold(#instance, |#instance, #call| #call(#instance))
                .create()
        }
    } else {
        quote!(::core::unreachable!())
    };
    let async_reporter = asynchronous.then(async_reporter);
    let doc = format!(
        "Starts creating an instance of `{workflow}`, for one journey: set each field it \
         requires, then, if any, its initial data and the reporters made for this journey, and \
         create it."
    );
    quote! {
        impl #workflow {
            #[doc = #doc]
            #visibility fn builder() -> __ItineraBuilder<(#(#unset,)*)> {
                __ItineraBuilder {
                    fields: (#(#unset,)*),
                    calls: ::std::vec::Vec::new(),
                }
            }
        }

        #[doc = "Creates an instance of the workflow."]
        pub struct __ItineraBuilder<__ItineraFields> {
            fields: __ItineraFields,
            calls: ::std::vec::Vec<
                ::std::boxed::Box<dyn ::core::ops::FnOnce(#instance_builder) -> #instance_builder>,
            >,
        }

        #[doc = "A field not set yet."]
        pub struct __ItineraUnset;

        #[doc = "A field set to this value."]
        pub struct __ItineraSet<T> {
            value: T,
        }

        #(#traits)*

        impl<#(#states),*> __ItineraBuilder<(#(#states,)*)> {
            #(#setters)*

            #[doc = "Gives the journey its initial data; without it, the data bag starts empty."]
            pub fn initial_data(
                mut self,
                data: impl ::itinera::journey::InitialData + 'static,
            ) -> Self {
                self.calls.push(::std::boxed::Box::new(move |instance: #instance_builder| {
                    instance.initial_data(data)
                }));
                self
            }

            #[doc = "Adds a reporter made for this journey, after the workflow's listed reporters."]
            pub fn reporter(mut self, reporter: impl ::itinera::report::Reporter) -> Self {
                self.calls.push(::std::boxed::Box::new(move |instance: #instance_builder| {
                    instance.reporter(reporter)
                }));
                self
            }

            #async_reporter

            #[doc = "Creates the instance, with its journey ID and its reporters."]
            #[doc = ""]
            #[doc = "# Errors"]
            #[doc = ""]
            #[doc = "When the journey ID cannot be produced, or a reporter fails to take it."]
            pub fn create(
                self,
            ) -> ::core::result::Result<
                ::itinera::instance::Instance<#workflow, #mode>,
                ::itinera::instance::InstanceError,
            >
            where
                #(#states: #bounds,)*
            {
                #created
            }
        }
    }
}

fn async_reporter() -> TokenStream {
    quote! {
        #[doc = "Adds an asynchronous reporter made for this journey, after the workflow's listed reporters."]
        pub fn async_reporter(mut self, reporter: impl ::itinera::report::AsyncReporter) -> Self {
            self.calls.push(::std::boxed::Box::new(move |instance| instance.async_reporter(reporter)));
            self
        }
    }
}

/// The builder's type parameter for the state of the field at this index.
fn state(index: usize) -> Ident {
    format_ident!("__ItineraState{index}")
}

/// The trait through which `create()` takes the field at this index.
fn bound(index: usize) -> Ident {
    format_ident!("__ItineraField{index}")
}

/// The variable holding the state of the field at this index.
fn variable(index: usize) -> Ident {
    local(&format!("field_{index}"))
}
