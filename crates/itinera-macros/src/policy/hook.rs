//! The hooks a policy defines: which hook each method is, what it requests, and the hook trait's
//! implementation that calls it.

mod name;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{ImplItemFn, PatType, Receiver, ReceiverKind, ReturnType, Signature, Type};

use super::request::Request;
use crate::{block, described};
pub(crate) use name::HookName;

/// What a parameter of the hook requests, and where its type is written, so that a request of
/// another type is reported there.
struct Parameter {
    request: Request,
    ty: Span,
}

impl Parameter {
    /// Reads what the parameter requests, and removes its attribute.
    fn read(parameter: &mut PatType, hook: HookName) -> syn::Result<Self> {
        Ok(Self {
            request: Request::read(parameter, hook)?,
            ty: parameter.ty.span(),
        })
    }

    fn request(&self) -> &Request {
        &self.request
    }

    /// The variable holding the request at this index.
    fn argument(&self, index: usize) -> syn::Ident {
        format_ident!("request_{index}", span = self.ty)
    }
}

/// A method marked as a hook, and what it requests, in the order of its parameters.
pub(super) struct Hook {
    name: HookName,
    method: syn::Ident,
    asynchronous: Option<Span>,
    parameters: Vec<Parameter>,
    /// Where the method's return type is written, so that a type the hook does not return is
    /// reported there.
    output: Span,
}

impl Hook {
    pub(super) fn defines(&self, name: HookName) -> bool {
        self.name == name
    }

    pub(super) fn is_asynchronous(&self) -> bool {
        self.asynchronous.is_some()
    }

    pub(super) fn differs_in_mode_from(&self, other: &Self) -> bool {
        self.is_asynchronous() != other.is_asynchronous()
    }

    /// Where the hook's method is named.
    pub(super) fn span(&self) -> Span {
        self.method.span()
    }

    /// The hook's name in upper camel case, such as `OnStepSuccess`.
    pub(super) fn variant(&self) -> syn::Ident {
        self.name.variant()
    }

    /// The name of the hook's method on the descriptor, such as `on_step_success`.
    pub(super) fn method(&self) -> syn::Ident {
        self.name.method()
    }

    /// Reads the hook's requests, and removes their attributes.
    pub(super) fn read(name: HookName, function: &mut ImplItemFn) -> syn::Result<Self> {
        if !function.sig.receiver().is_some_and(by_reference) {
            return Err(syn::Error::new_spanned(
                &function.sig,
                "a hook takes the policy as `&self`",
            ));
        }
        let asynchronous = function.sig.asyncness.map(|asynchronous| asynchronous.span);
        if let Some(span) = asynchronous {
            if !cfg!(feature = "async") {
                return Err(syn::Error::new(
                    span,
                    "an asynchronous hook needs the `async` feature of `itinera`",
                ));
            }
        }
        let parameters = function
            .sig
            .inputs
            .iter_mut()
            .filter_map(block::typed)
            .map(|parameter| Parameter::read(parameter, name))
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            name,
            method: function.sig.ident.clone(),
            asynchronous,
            parameters,
            output: returned(&function.sig),
        })
    }

    /// The roles the hook requests.
    pub(super) fn roles(&self) -> impl Iterator<Item = &Type> {
        self.requests().filter_map(Request::role)
    }

    fn requests(&self) -> impl Iterator<Item = &Request> {
        self.parameters.iter().map(Parameter::request)
    }

    /// The hook trait's implementation, which resolves the requests and calls the method.
    pub(super) fn implementation(
        &self,
        policy: &Type,
        workflow: &syn::Ident,
        bounds: &TokenStream,
    ) -> TokenStream {
        let hook_trait = match self.asynchronous {
            Some(_) => format_ident!("Async{}", self.name.variant()),
            None => self.name.variant(),
        };
        let kind = self.name.hook_kind();
        let (asyncness, awaited) = match self.asynchronous {
            Some(_) => (quote!(async), quote!(.await)),
            None => (quote!(), quote!()),
        };
        let mode = described::mode(self.is_asynchronous());
        let needs = self.requests().filter_map(Request::need);
        let got = format_ident!("got");
        let pattern = if self.parameters.is_empty() {
            quote!(_)
        } else if self.requests().any(Request::takes) {
            quote!(mut #got)
        } else {
            quote!(#got)
        };
        let arguments: Vec<syn::Ident> = self
            .parameters
            .iter()
            .enumerate()
            .map(|(index, parameter)| parameter.argument(index))
            .collect();
        let taken = self.requests().map(|request| request.taken(&got));
        let hook = self.name.method();
        let mut method = self.method.clone();
        method.set_span(self.output);
        let returned = self.name.returned();
        let called =
            quote_spanned!(self.output=> <#policy>::#method(self, #(#arguments),*) #awaited);
        quote! {
            impl<#workflow> ::itinera::policy::#hook_trait<#workflow> for #policy
            where
                #workflow: ::core::marker::Send + ::core::marker::Sync + 'static #bounds
            {
                fn needs() -> ::itinera::policy::HookNeeds<::itinera::policy::#kind> {
                    ::itinera::policy::HookNeeds::new() #(#needs)*
                }

                #asyncness fn #hook(
                    &self,
                    #pattern: ::itinera::policy::Requested<'_, #workflow, ::itinera::policy::#kind, #mode>,
                ) -> ::core::result::Result<#returned, ::itinera::error::Error> {
                    #(let #arguments = #taken;)*
                    #called
                }
            }
        }
    }
}

/// Where the method's return type is written, or its name when it has none.
fn returned(signature: &Signature) -> Span {
    match &signature.output {
        ReturnType::Type(_, ty) => ty.span(),
        ReturnType::Default => signature.ident.span(),
    }
}

fn by_reference(receiver: &Receiver) -> bool {
    matches!(receiver.kind, ReceiverKind::Reference(_, _, None))
}
