//! `#[step_policy]` and `#[workflow_policy]`: a policy's hooks, what each requests, and how the
//! policy is built.

mod factory;
mod hook;
mod kind;
mod marked;
mod name;
mod request;
mod stand_in;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemImpl;

use crate::{block, described, marker};
use factory::Factory;
use hook::Hook;
pub(crate) use hook::HookName;
pub(crate) use kind::Kind;
use marked::Marked;

/// The attributes that mark a function of the block: each hook's, and the factory's.
pub(crate) const MARKS: [&str; 7] = [
    HookName::OnStepSuccess.marker(),
    HookName::OnStepFailure.marker(),
    HookName::OnStepRetry.marker(),
    HookName::OnStepAbnormalTermination.marker(),
    HookName::OnWorkflowSuccess.marker(),
    HookName::OnWorkflowFailure.marker(),
    "factory",
];

/// The shorthands for what a step hook returns that `DescribedPolicy` gives.
const SHORTHANDS: [&str; 3] = ["keep_default", "fail_workflow", "finish_workflow"];

/// The generated implementations' parameter for the workflow, named so that it clashes with no
/// type the application names.
fn workflow() -> syn::Ident {
    format_ident!("__ItineraWorkflow")
}

/// Whether a function of the block is marked as a hook or as the factory.
fn is_marked(function: &mut syn::ImplItemFn) -> bool {
    !marker::found(&function.attrs, &MARKS).is_empty()
}

/// Whether a function of the block is marked as a hook and is asynchronous.
fn is_asynchronous_hook(function: &mut syn::ImplItemFn) -> bool {
    function.sig.asyncness.is_some() && function.attrs.iter().any(marks_a_hook)
}

fn marks_a_hook(attribute: &syn::Attribute) -> bool {
    marker::name(attribute).parse::<HookName>().is_ok()
}

/// Implements `DescribedPolicy`, the policy trait of its kind and a hook trait per hook for the
/// type of the block. On an error, it implements stand-ins instead, so that the compiler reports
/// that error alone.
pub(crate) fn expand(kind: Kind, arguments: TokenStream, mut block: ItemImpl) -> TokenStream {
    let marked: Vec<bool> = block::functions(&mut block).map(is_marked).collect();
    let asynchronous =
        cfg!(feature = "async") && block::functions(&mut block).any(is_asynchronous_hook);
    let described = describe(kind, arguments, &mut block);
    let import = quote!(
        use ::itinera::policy::DescribedPolicy as _;
    );
    let marks: Vec<&str> = MARKS.into_iter().chain(request::MARKS).collect();
    for (function, marked) in block::functions(&mut block).zip(marked) {
        if marked {
            block::unmark(function, &marks);
        }
        block::bring_into_scope(function, &import, &SHORTHANDS);
    }
    match described {
        Ok(described) => quote! {
            #block
            #described
        },
        Err(error) => {
            let error = error.into_compile_error();
            let stand_in = stand_in::stand_in(kind, &block, asynchronous);
            quote! {
                #block
                #error
                #stand_in
            }
        }
    }
}

fn describe(kind: Kind, arguments: TokenStream, block: &mut ItemImpl) -> syn::Result<TokenStream> {
    let attribute = kind.attribute();
    let name = name::read(arguments, attribute, &block.self_ty)?;
    if let Some((path, _)) = &block.trait_ {
        return Err(syn::Error::new_spanned(
            path,
            format!(
                "`{attribute}` goes on the policy's own `impl` block, not on a trait implementation"
            ),
        ));
    }
    described::refuse_generics(&block.generics, "policy")?;
    let Marked { hooks, factory } = Marked::read(block, kind)?;
    let mode = mode(&hooks, &block.self_ty, attribute)?;
    let policy = &block.self_ty;
    let roles = hooks.iter().flat_map(Hook::roles);
    let workflow = workflow();
    let bounds = quote!(#(, #workflow: ::itinera::policy::Provides<#roles>)*);
    let implementations = hooks
        .iter()
        .map(|hook| hook.implementation(policy, &workflow, &bounds));
    let (policy_trait, descriptor, hook_type) = kind.names();
    let asynchronous = hooks.iter().any(Hook::is_asynchronous);
    let constructor = if asynchronous {
        format_ident!("fallible_async")
    } else {
        format_ident!("fallible")
    };
    let variants = hooks.iter().map(Hook::variant);
    let defined = hooks.iter().map(Hook::method);
    let build = Factory::build(factory.as_ref(), policy);
    Ok(quote! {
        const _: () = {
            impl ::itinera::policy::DescribedPolicy for #policy {
                type Mode = #mode;

                fn name() -> ::itinera::policy::PolicyName {
                    ::itinera::policy::PolicyName::from(#name)
                }

                fn build() -> ::core::result::Result<Self, ::itinera::error::Error> {
                    #build
                }
            }

            impl<#workflow> ::itinera::policy::#policy_trait<#workflow> for #policy
            where
                #workflow: ::core::marker::Send + ::core::marker::Sync + 'static #bounds
            {
                const HOOKS: &'static [::itinera::policy::#hook_type] =
                    &[#(::itinera::policy::#hook_type::#variants),*];

                fn descriptor() -> ::itinera::policy::#descriptor<Self, #workflow, #mode> {
                    ::itinera::policy::#descriptor::#constructor(
                        <Self as ::itinera::policy::DescribedPolicy>::name(),
                        <Self as ::itinera::policy::DescribedPolicy>::build,
                    ) #(.#defined())*
                }
            }

            #(#implementations)*
        };
    })
}

/// The mode of the policy's hooks, which are all synchronous or all asynchronous.
fn mode(hooks: &[Hook], policy: &syn::Type, attribute: &str) -> syn::Result<TokenStream> {
    let Some(first) = hooks.first() else {
        return Err(syn::Error::new_spanned(
            policy,
            format!(
                "a policy described by `{attribute}` defines at least one hook, marked such as `#[on_step_success]`"
            ),
        ));
    };
    if let Some(other) = hooks.iter().find(|hook| hook.differs_in_mode_from(first)) {
        return Err(syn::Error::new(
            other.span(),
            "a policy's hooks are all synchronous, or all asynchronous",
        ));
    }
    Ok(described::mode(first.is_asynchronous()))
}
