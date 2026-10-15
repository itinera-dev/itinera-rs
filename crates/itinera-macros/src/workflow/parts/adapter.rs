//! An input adapter of the workflow's `impl` block: a method marked
//! `#[input_adapter(steps = [...])]`, the steps it is attached to, and what it requests.

use std::iter;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parser as _;
use syn::{Attribute, Ident, ImplItemFn, LitStr, Type};

mod request;

use super::{arguments, refuse_other_receivers_and_modes, returned};
use crate::local::local;
use crate::workflow::{key, list, once};
use crate::{block, described};
pub(super) use request::MARKS;
use request::Request;

/// The shorthands for what an input adapter returns that `Workflow` gives.
const SHORTHANDS: [&str; 2] = ["supply", "from_data_bag"];

/// Starts the function's body with an import of `Workflow` when the body writes one of the
/// shorthands, such as `Self::supply`, so that it finds them.
pub(super) fn bring_shorthands_into_scope(function: &mut ImplItemFn) {
    let import = quote!(
        use ::itinera::workflow::Workflow as _;
    );
    block::bring_into_scope(function, &import, &SHORTHANDS);
}

/// A method marked as an input adapter.
pub(super) struct Adapter {
    name: LitStr,
    /// The first step the adapter is attached to, which its descriptor is made for.
    first: LitStr,
    /// The other steps it is attached to.
    others: Vec<LitStr>,
    method: Ident,
    requests: Vec<Request>,
    /// Where the method's return type is written, so that a type an adapter does not return is
    /// reported there.
    output: Span,
}

impl Adapter {
    /// Reads the adapter its mark describes, and removes its parameters' attributes.
    pub(super) fn read(function: &mut ImplItemFn, mark: &Attribute) -> syn::Result<Self> {
        let mut name = None;
        let mut steps = None;
        syn::meta::parser(|meta| match key(&meta).as_str() {
            "name" => once(&mut name, meta.value()?.parse()?, &meta),
            "steps" => once(&mut steps, list::<LitStr>(&meta)?, &meta),
            _ => Err(meta.error(MISREAD)),
        })
        .parse2(arguments(mark, MISREAD)?)?;
        let mut steps = steps.into_iter().flatten();
        let Some(first) = steps.next() else {
            return Err(syn::Error::new_spanned(
                mark,
                "an input adapter is attached to at least one step, as in \
                 `#[input_adapter(steps = [\"charge\"])]`",
            ));
        };
        let others = steps.collect();
        refuse_other_receivers_and_modes(function)?;
        let requests = function
            .sig
            .inputs
            .iter_mut()
            .filter_map(block::typed)
            .map(Request::read)
            .collect::<syn::Result<_>>()?;
        let method = function.sig.ident.clone();
        Ok(Self {
            name: name.unwrap_or_else(|| described::named_after(&method)),
            first,
            others,
            method,
            requests,
            output: returned(function),
        })
    }

    pub(super) fn name(&self) -> &LitStr {
        &self.name
    }

    pub(super) fn shares_name_with(&self, other: &Self) -> bool {
        self.name.value() == other.name.value()
    }

    /// The adapter with each step it is attached to.
    pub(super) fn attachments(&self) -> impl Iterator<Item = (&Self, &LitStr)> {
        iter::repeat(self).zip(iter::once(&self.first).chain(&self.others))
    }

    /// The function the descriptor calls, which resolves the requests and calls the method.
    pub(super) fn function(&self, workflow: &Type) -> TokenStream {
        let function = self.function_name();
        let needs_got = self.requests.iter().any(Request::takes);
        let got = local("got");
        let workflow_value = local("workflow");
        let pattern = if self.requests.is_empty() {
            quote!(_)
        } else if needs_got {
            quote!(mut #got)
        } else {
            quote!(#got)
        };
        let arguments: Vec<Ident> = (0..self.requests.len()).map(argument).collect();
        let taken = self.requests.iter().map(|request| request.taken(&got));
        let mut method = self.method.clone();
        method.set_span(self.output);
        let called =
            quote_spanned!(self.output=> <#workflow>::#method(#workflow_value, #(#arguments),*));
        quote! {
            fn #function(
                #workflow_value: &#workflow,
                #pattern: ::itinera::policy::Requested<'_, #workflow, ::itinera::policy::InputAdapter>,
            ) -> ::core::result::Result<::itinera::workflow::Supplied, ::itinera::error::Error> {
                #(let #arguments = #taken;)*
                #called
            }
        }
    }

    /// The adapter's descriptor, attached to its steps, with what it needs.
    pub(super) fn descriptor(&self) -> TokenStream {
        let name = &self.name;
        let function = self.function_name();
        let needs = self.requests.iter().filter_map(Request::need);
        let Self { first, others, .. } = self;
        quote! {
            ::itinera::workflow::InputAdapterDescriptor::new(
                #name,
                ::itinera::step::step_name!(#first),
                #function,
            )
            .needing(::itinera::policy::HookNeeds::new() #(#needs)*)
            #(.step(::itinera::step::step_name!(#others)))*
        }
    }

    fn function_name(&self) -> Ident {
        format_ident!("__itinera_input_adapter_{}", self.method)
    }
}

/// What `#[input_adapter]` takes, as its errors say it.
const MISREAD: &str =
    "`#[input_adapter]` takes `steps = [\"...\"]`, and optionally `name = \"...\"`";

/// The variable holding the request at this index.
fn argument(index: usize) -> Ident {
    local(&format!("request_{index}"))
}
