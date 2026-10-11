//! The method marked `#[id_generator]`, which produces the journey ID of each new instance.

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Ident, ImplItemFn, Type};

use super::{refuse_other_receivers_and_modes, returned};
use crate::local::local;

/// The ID generator, and whether it reads the initial data.
pub(super) struct IdGenerator {
    method: Ident,
    reads_data: bool,
    /// Where the method's return type is written.
    output: Span,
}

impl IdGenerator {
    pub(super) fn read(function: &ImplItemFn) -> syn::Result<Self> {
        refuse_other_receivers_and_modes(function)?;
        let mut parameters = function.sig.inputs.iter().skip(1);
        let reads_data = parameters.next().is_some();
        if let Some(another) = parameters.next() {
            return Err(syn::Error::new_spanned(
                another,
                "an ID generator takes `&self`, and the initial data as `&DataBag` if it reads it",
            ));
        }
        Ok(Self {
            method: function.sig.ident.clone(),
            reads_data,
            output: returned(function),
        })
    }

    /// The block's ID generator, if it has one, or an error at the second.
    pub(super) fn only<'a>(
        mut generators: impl Iterator<Item = &'a Self>,
    ) -> syn::Result<Option<&'a Self>> {
        let generator = generators.next();
        match generators.next() {
            Some(another) => Err(syn::Error::new(
                another.method.span(),
                "a workflow has one ID generator",
            )),
            None => Ok(generator),
        }
    }

    /// The builder's call that makes the method the workflow's ID generator.
    pub(super) fn declared(&self, workflow: &Type) -> TokenStream {
        let mut method = self.method.clone();
        method.set_span(self.output);
        let workflow_value = local("workflow");
        let data = local("data");
        let (pattern, given) = if self.reads_data {
            (quote!(#data), quote!(#data))
        } else {
            (quote!(_), quote!())
        };
        let called = quote_spanned!(self.output=> <#workflow>::#method(#workflow_value, #given));
        quote! {
            .id_generator(|#workflow_value: &#workflow, #pattern: &::itinera::journey::DataBag| #called)
        }
    }
}
