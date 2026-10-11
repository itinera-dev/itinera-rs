//! The variables of the generated code, which the application's code can neither name nor
//! shadow, even when its own names are the same.

use proc_macro2::{Ident, Span};

/// A variable of the generated code with this name.
pub(crate) fn local(name: &str) -> Ident {
    Ident::new(name, Span::mixed_site())
}
