//! The instance builder `#[workflow]` gives the workflow: one setter per field of its struct,
//! then its initial data, initial global metadata and reporters, and `create()`, which compiles
//! only once every required field is set.

mod field;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Visibility};

pub(super) use field::{Settable, settable};

use super::mode;
use crate::local::local;

/// `Workflow::builder()`, the builder's type, and the trait through which `create()` takes each
/// field, for a workflow of this type and mode with these fields. When some of its fields could
/// not be read, `create()` is a stand-in that is never called.
pub(super) fn builder(
    workflow: &Ident,
    visibility: &Visibility,
    fields: &[Settable],
    mode: &TokenStream,
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
    let async_reporters = mode::asynchronous_reporters(workflow);
    let async_reporter = async_reporters.is_some().then(|| async_reporter(mode));
    let doc = format!(
        "Starts creating an instance of `{workflow}`, for one journey: set each field it \
         requires, then, if any, its initial data, its initial global metadata and the reporters made \
         for this journey, and create it."
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

        #async_reporters

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

            #[doc = "Adds an entry of initial global metadata, which every reporter receives before the journey starts, right after the journey ID. Setting a key again replaces its value; setting `journey_id` makes `run` refuse the journey."]
            pub fn reporters_global_metadata<__ItineraValue: ::itinera::value::Value>(
                mut self,
                key: impl ::core::convert::Into<::std::string::String>,
                value: __ItineraValue,
            ) -> Self {
                let key = key.into();
                self.calls.push(::std::boxed::Box::new(move |instance: #instance_builder| {
                    instance.reporters_global_metadata(key, value)
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
            #[doc = "When the journey ID cannot be produced."]
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

/// The builder's `async_reporter`, which only an asynchronous workflow's builder takes a reporter
/// in.
fn async_reporter(mode: &TokenStream) -> TokenStream {
    quote! {
        #[doc = "Adds an asynchronous reporter made for this journey, after the workflow's listed reporters, in an asynchronous workflow."]
        pub fn async_reporter(
            mut self,
            reporter: impl __ItineraAsyncReporter<#mode> + 'static,
        ) -> Self {
            self.calls.push(::std::boxed::Box::new(move |instance| reporter.__itinera_added(instance)));
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
