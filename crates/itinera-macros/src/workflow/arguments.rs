//! The arguments of `#[workflow(...)]` on a struct: the workflow's name, its policies and
//! reporters, and its mode.

use proc_macro2::{Span, TokenStream};
use syn::parse::Parser as _;
use syn::{LitStr, Type};

use super::{key, list, once};

/// What `#[workflow(...)]` says, every part of it optional.
#[derive(Default)]
pub(super) struct Arguments {
    pub(super) name: Option<LitStr>,
    pub(super) policies: Vec<Type>,
    pub(super) reporters: Vec<Type>,
    pub(super) async_reporters: Vec<Type>,
    /// Where `asynchronous` is written, if it is.
    pub(super) asynchronous: Option<Span>,
}

impl Arguments {
    pub(super) fn read(arguments: TokenStream) -> syn::Result<Self> {
        let mut name = None;
        let mut policies = None;
        let mut reporters = None;
        let mut async_reporters = None;
        let mut asynchronous = None;
        syn::meta::parser(|meta| match key(&meta).as_str() {
            "name" => once(&mut name, meta.value()?.parse()?, &meta),
            "policies" => once(&mut policies, list(&meta)?, &meta),
            "reporters" => once(&mut reporters, list(&meta)?, &meta),
            "async_reporters" => once(&mut async_reporters, list(&meta)?, &meta),
            "asynchronous" => once(&mut asynchronous, span(&meta), &meta),
            _ => Err(meta.error(
                "`#[workflow]` on a struct takes `name = \"...\"`, `policies = [...]`, \
                     `reporters = [...]`, `async_reporters = [...]` and `asynchronous`, each \
                     optional",
            )),
        })
        .parse2(arguments)?;
        let arguments = Self {
            name,
            policies: policies.into_iter().flatten().collect(),
            reporters: reporters.into_iter().flatten().collect(),
            async_reporters: async_reporters.into_iter().flatten().collect(),
            asynchronous,
        };
        arguments.check_mode()?;
        Ok(arguments)
    }

    pub(super) fn is_asynchronous(&self) -> bool {
        self.asynchronous.is_some()
    }

    /// Refuses asynchronous reporters in a synchronous workflow, and an asynchronous workflow
    /// without the `async` feature.
    fn check_mode(&self) -> syn::Result<()> {
        match (self.asynchronous, self.async_reporters.first()) {
            (None, Some(reporter)) => Err(syn::Error::new_spanned(
                reporter,
                "asynchronous reporters belong to an asynchronous workflow: add `asynchronous` to \
                 `#[workflow(...)]`",
            )),
            (Some(span), _) if !cfg!(feature = "async") => Err(syn::Error::new(
                span,
                "an asynchronous workflow needs the `async` feature of `itinera`",
            )),
            _ => Ok(()),
        }
    }
}

fn span(meta: &syn::meta::ParseNestedMeta<'_>) -> Span {
    syn::spanned::Spanned::span(&meta.path)
}
