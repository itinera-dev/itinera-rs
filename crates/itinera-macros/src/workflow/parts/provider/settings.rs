//! The settings of `#[provider(...)]`: the provider's scope, the steps it is attached to, the key
//! of the service it supplies and its name.

use proc_macro2::TokenStream;
use syn::LitStr;
use syn::parse::Parser as _;

use crate::workflow::{key, list, once};

/// The settings of `#[provider(...)]`, each optional.
#[derive(Default)]
pub(super) struct Settings {
    pub(super) name: Option<LitStr>,
    pub(super) key: Option<LitStr>,
    pub(super) per_attempt: Option<()>,
    pub(super) steps: Option<Vec<LitStr>>,
}

/// What `#[provider]` takes, as its errors say it.
pub(super) const MISREAD: &str = "`#[provider]` takes, each optional, `per_attempt`, `steps = [\"...\"]`, \
                       `key = \"...\"` and `name = \"...\"`";

impl Settings {
    /// Reads the settings from the tokens inside the mark's parentheses.
    pub(super) fn read(arguments: TokenStream) -> syn::Result<Self> {
        let mut settings = Self::default();
        syn::meta::parser(|meta| settings.setting(&meta)).parse2(arguments)?;
        Ok(settings)
    }

    fn setting(&mut self, meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
        match key(meta).as_str() {
            "name" => once(&mut self.name, meta.value()?.parse()?, meta),
            "key" => once(&mut self.key, meta.value()?.parse()?, meta),
            "per_attempt" => once(&mut self.per_attempt, (), meta),
            "steps" => once(
                &mut self.steps,
                list::<LitStr>(meta)?.into_iter().collect(),
                meta,
            ),
            _ => Err(meta.error(MISREAD)),
        }
    }
}
