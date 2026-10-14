//! A field of the workflow's struct: how the builder sets it, and how `create()` takes it.

use proc_macro2::TokenStream;
use quote::quote;
use syn::ext::IdentExt as _;
use syn::{Field, Fields, Ident, Type};

use super::{bound, variable};
use crate::datum::option_of;
use crate::marker;

/// The builder's own methods, which no field may be named after.
const METHODS: [&str; 6] = [
    "initial_data",
    "reporters_global_metadata",
    "reporter",
    "async_reporter",
    "fake_provider",
    "create",
];

/// A field of the workflow's struct, which its builder sets.
pub(in crate::workflow) struct Settable {
    name: Ident,
    ty: Type,
    /// Whether the field starts from its default when it is not set: an `Option`, or a field
    /// marked `#[default]`.
    defaulted: bool,
}

/// The struct's fields, which the builder sets, each read and stripped of its `#[default]` mark,
/// or the error that reading it gave.
pub(in crate::workflow) fn settable(fields: &mut Fields) -> Vec<syn::Result<Settable>> {
    match fields {
        Fields::Named(fields) => fields.named.iter_mut().map(Settable::read).collect(),
        Fields::Unit => Vec::new(),
        Fields::Unnamed(fields) => fields.unnamed.first().map(unnamed).into_iter().collect(),
    }
}

fn unnamed(field: &Field) -> syn::Result<Settable> {
    Err(syn::Error::new_spanned(
        field,
        "a workflow's fields have names, which its builder's setters take",
    ))
}

impl Settable {
    fn read(field: &mut Field) -> syn::Result<Self> {
        let marks = marker::take(&mut field.attrs, &["default"]);
        let marked = marker::at_most_one(marks, "a field is marked `#[default]` once")?;
        if let Some(mark) = &marked {
            mark.meta.require_path_only()?;
        }
        let Some(name) = field.ident.clone() else {
            return Err(syn::Error::new_spanned(
                &field,
                "a workflow's fields have names, which its builder's setters take",
            ));
        };
        if METHODS.contains(&name.unraw().to_string().as_str()) {
            return Err(syn::Error::new_spanned(
                &name,
                format!(
                    "a workflow's field is not named `{}`, which its builder's own method is \
                     named: rename the field",
                    name.unraw()
                ),
            ));
        }
        Ok(Self {
            defaulted: marked.is_some() || option_of(&field.ty).is_some(),
            name,
            ty: field.ty.clone(),
        })
    }
}

impl Settable {
    /// The trait through which `create()` takes the field's value: set, or its default when the
    /// field has one.
    pub(super) fn taken(&self, workflow: &Ident, index: usize) -> TokenStream {
        let Self { name, ty, .. } = self;
        let field = bound(index);
        let name = name.unraw();
        let message = format!("`{workflow}::builder()` is not given `{name}`, which it requires");
        let label = format!("`{name}` is not set");
        let note = format!(
            "call `.{name}(...)` before `.create()`, or mark the field `#[default]` so that it \
             starts from its default"
        );
        let default = self.defaulted.then(|| defaulted(&field, ty));
        quote! {
            #[doc = "Gives `create()` a field's value."]
            #[diagnostic::on_unimplemented(message = #message, label = #label, note = #note)]
            pub trait #field {
                #[doc = "The field's value."]
                fn value(self) -> #ty;
            }

            impl #field for __ItineraSet<#ty> {
                fn value(self) -> #ty {
                    self.value
                }
            }

            #default
        }
    }

    /// The setter of the field at this index, which sets it in the builder's state.
    pub(super) fn setter(&self, index: usize, states: &[Ident]) -> TokenStream {
        let Self { name, ty, .. } = self;
        let set = states
            .iter()
            .enumerate()
            .map(|(other, state)| set_or_kept(other, index, state, ty));
        let ignored = quote!(_);
        let set_now = quote!(__ItineraSet { value: #name });
        let taken = (0..states.len()).map(|other| kept(other, index, &ignored));
        let given = (0..states.len()).map(|other| kept(other, index, &set_now));
        let doc = format!("Sets `{}`.", name.unraw());
        quote! {
            #[doc = #doc]
            pub fn #name(self, #name: #ty) -> __ItineraBuilder<(#(#set,)*)> {
                let (#(#taken,)*) = self.fields;
                __ItineraBuilder {
                    fields: (#(#given,)*),
                    calls: self.calls,
                }
            }
        }
    }

    /// The field, given its value from the builder's state at this index.
    pub(super) fn assigned(&self, index: usize) -> TokenStream {
        let name = &self.name;
        let field = bound(index);
        let variable = variable(index);
        quote!(#name: #field::value(#variable))
    }
}

/// The field's default, for `create()` when the field is not set.
fn defaulted(field: &Ident, ty: &Type) -> TokenStream {
    quote! {
        impl #field for __ItineraUnset {
            fn value(self) -> #ty {
                <#ty as ::core::default::Default>::default()
            }
        }
    }
}

/// The state of the field at `other` once the field at `index` is set.
fn set_or_kept(other: usize, index: usize, state: &Ident, ty: &Type) -> TokenStream {
    if other == index {
        quote!(__ItineraSet<#ty>)
    } else {
        quote!(#state)
    }
}

/// The variable holding the state of the field at this index, or this in place of the field
/// being set.
fn kept(other: usize, index: usize, set: &TokenStream) -> TokenStream {
    if other == index {
        set.clone()
    } else {
        let variable = variable(other);
        quote!(#variable)
    }
}
