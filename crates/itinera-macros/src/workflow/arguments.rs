//! The arguments of `#[workflow(...)]` on a struct: the workflow's name, its policies and its
//! reporters.

use proc_macro2::TokenStream;
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
}

impl Arguments {
    pub(super) fn read(arguments: TokenStream) -> syn::Result<Self> {
        let mut name = None;
        let mut policies = None;
        let mut reporters = None;
        let mut async_reporters = None;
        syn::meta::parser(|meta| match key(&meta).as_str() {
            "name" => once(&mut name, meta.value()?.parse()?, &meta),
            "policies" => once(&mut policies, list(&meta)?, &meta),
            "reporters" => once(&mut reporters, list(&meta)?, &meta),
            "async_reporters" => once(&mut async_reporters, list(&meta)?, &meta),
            _ => Err(meta.error(
                "`#[workflow]` on a struct takes `name = \"...\"`, `policies = [...]`, \
                     `reporters = [...]` and `async_reporters = [...]`, each optional",
            )),
        })
        .parse2(arguments)?;
        let arguments = Self {
            name,
            policies: policies.into_iter().flatten().collect(),
            reporters: reporters.into_iter().flatten().collect(),
            async_reporters: async_reporters.into_iter().flatten().collect(),
        };
        arguments.refuse_async_reporters_without_the_feature()?;
        Ok(arguments)
    }

    /// Refuses asynchronous reporters without the `async` feature. With it, a workflow whose
    /// steps are synchronous refuses them as it compiles.
    fn refuse_async_reporters_without_the_feature(&self) -> syn::Result<()> {
        match self.async_reporters.first() {
            Some(reporter) if !cfg!(feature = "async") => Err(syn::Error::new_spanned(
                reporter,
                "asynchronous reporters need the `async` feature of `itinera`",
            )),
            _ => Ok(()),
        }
    }
}
