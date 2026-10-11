//! The functions the generated checks call as the program compiles, to find a hook defined twice
//! or an input adapter for an unknown step. Each is generated into a module of its own, only
//! when a check calls it.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Ident, Type};

use crate::policy::{HookName, Kind};

/// `__itinera_step_hooks` or `__itinera_workflow_hooks`, whose `twice` says whether two lists of
/// hooks, such as the `HOOKS` of a step's policies, hold the same hook.
pub(super) fn hooks(kind: Kind) -> TokenStream {
    let (module, hook) = names(kind);
    let variants: Vec<syn::Ident> = HookName::ALL
        .into_iter()
        .filter(|name| name.is_of(kind))
        .map(HookName::variant)
        .collect();
    quote! {
        mod #module {
            pub(super) type Hook = ::itinera::policy::#hook;

            pub(super) const fn twice(lists: &[&[Hook]], hook: Hook) -> bool {
                match lists {
                    [] => false,
                    [first, rest @ ..] => (holds(first, hook) && held(rest, hook)) || twice(rest, hook),
                }
            }

            const fn held(lists: &[&[Hook]], hook: Hook) -> bool {
                match lists {
                    [] => false,
                    [first, rest @ ..] => holds(first, hook) || held(rest, hook),
                }
            }

            const fn holds(list: &[Hook], hook: Hook) -> bool {
                match list {
                    [] => false,
                    [first, rest @ ..] => same(*first, hook) || holds(rest, hook),
                }
            }

            const fn same(one: Hook, other: Hook) -> bool {
                ::core::matches!((one, other), #((Hook::#variants, Hook::#variants))|*)
            }
        }
    }
}

/// The module of the functions that check hooks of this kind, and the type of those hooks.
fn names(kind: Kind) -> (Ident, Ident) {
    match kind {
        Kind::Step => (
            format_ident!("__itinera_step_hooks"),
            format_ident!("StepHook"),
        ),
        Kind::Workflow => (
            format_ident!("__itinera_workflow_hooks"),
            format_ident!("WorkflowHook"),
        ),
    }
}

/// Policies of one kind attached together, to a step or to the workflow.
pub(super) struct Attached<'a> {
    pub(super) kind: Kind,
    /// What they are attached to, as the errors say it, such as `step charge`.
    pub(super) to: String,
    pub(super) workflow: &'a Ident,
    pub(super) policies: &'a [Type],
}

impl Attached<'_> {
    /// The checks, run as the program compiles, that no two of the policies define the same hook,
    /// one for each hook of their kind, when there are two policies or more. Each is reported at
    /// the second policy.
    pub(super) fn checked(&self) -> Option<TokenStream> {
        let [_, second, ..] = self.policies else {
            return None;
        };
        let span = second.span();
        let checks = HookName::ALL
            .into_iter()
            .filter(|name| name.is_of(self.kind))
            .map(|hook| self.defined_once(hook, span));
        Some(quote!(#(#checks)*))
    }

    fn defined_once(&self, hook: HookName, span: Span) -> TokenStream {
        let (module, _) = names(self.kind);
        let policy = match self.kind {
            Kind::Step => quote!(StepPolicy),
            Kind::Workflow => quote!(WorkflowPolicy),
        };
        let Self {
            workflow, policies, ..
        } = self;
        let variant = hook.variant();
        let message = format!(
            "hook defined twice on {}: {}, which two of its policies define",
            self.to,
            hook.spoken()
        );
        quote_spanned! {span=>
            const _: () = ::core::assert!(
                !#module::twice(
                    &[#(<#policies as ::itinera::policy::#policy<#workflow>>::HOOKS),*],
                    #module::Hook::#variant,
                ),
                #message
            );
        }
    }
}

/// `__itinera_steps`, whose `listed` says whether a name is among the workflow's step names.
pub(super) fn steps() -> TokenStream {
    quote! {
        mod __itinera_steps {
            pub(super) const fn listed(steps: &[&str], name: &str) -> bool {
                match steps {
                    [] => false,
                    [first, rest @ ..] => same(first.as_bytes(), name.as_bytes()) || listed(rest, name),
                }
            }

            const fn same(one: &[u8], other: &[u8]) -> bool {
                match (one, other) {
                    ([], []) => true,
                    ([first, rest @ ..], [second, others @ ..]) => *first == *second && same(rest, others),
                    _ => false,
                }
            }
        }
    }
}
