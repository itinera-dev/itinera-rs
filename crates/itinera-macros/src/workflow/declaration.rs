//! `#[workflow(...)]` on a struct: the workflow's descriptor, held in a hidden static, and its
//! instance builder.

mod descriptor;
mod names;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, ItemStruct};

use super::arguments::Arguments;
use super::builder::{self, Settable};
use super::checks::{self, Attached};
use super::with_step::WithStep;
use super::{mode, services};
use crate::policy::Kind;
use crate::{described, marker};

/// Declares the workflow the struct describes. On an error, it gives the workflow stand-ins of
/// what it would have, so that the compiler reports that error alone.
pub(super) fn expand(arguments: TokenStream, mut declared: ItemStruct) -> TokenStream {
    let marks = marker::take(&mut declared.attrs, &["with_step"]);
    let (readable, unreadable): (Vec<_>, Vec<_>) = builder::settable(&mut declared.fields)
        .into_iter()
        .partition(Result::is_ok);
    let fields: Vec<Settable> = readable.into_iter().flatten().collect();
    let unreadable = unreadable.into_iter().find_map(Result::err);
    let complete = unreadable.is_none();
    let declaration = match unreadable {
        None => declare(arguments, &declared, &marks, &fields),
        Some(error) => Err(error),
    };
    match declaration {
        Ok(declaration) => quote! {
            #declared
            #declaration
        },
        Err(error) => {
            let error = error.into_compile_error();
            let stand_in = stand_in(&declared, &marks, &fields, complete);
            quote! {
                #declared
                #error
                #stand_in
            }
        }
    }
}

fn declare(
    arguments: TokenStream,
    declared: &ItemStruct,
    marks: &[Attribute],
    fields: &[Settable],
) -> syn::Result<TokenStream> {
    let arguments = Arguments::read(arguments)?;
    described::refuse_generics(&declared.generics, "workflow")?;
    let workflow = &declared.ident;
    let steps = marks
        .iter()
        .map(WithStep::read)
        .collect::<syn::Result<Vec<_>>>()?;
    if steps.is_empty() {
        return Err(syn::Error::new_spanned(
            workflow,
            "a workflow lists at least one step, with a `#[with_step(...)]` line below \
             `#[workflow(...)]`",
        ));
    }
    names::refuse_duplicates(&steps)?;
    let mode = mode::named(workflow);
    let name = arguments
        .name
        .clone()
        .unwrap_or_else(|| described::named_after(workflow));
    let descriptor = descriptor::descriptor(workflow, &name, &arguments, &steps);
    let step_checks: Vec<TokenStream> = steps
        .iter()
        .filter_map(|step| step.checked(workflow))
        .collect();
    let step_hooks = (!step_checks.is_empty()).then(|| checks::hooks(Kind::Step));
    let workflow_checks = Attached {
        kind: Kind::Workflow,
        to: "the workflow".to_owned(),
        workflow,
        policies: &arguments.policies,
    }
    .checked();
    let workflow_hooks = workflow_checks
        .is_some()
        .then(|| checks::hooks(Kind::Workflow));
    let listed = listed(workflow, &steps);
    let services = services::checked(workflow, &steps);
    let builder = builder::builder(workflow, &declared.vis, fields, &mode, true);
    Ok(quote! {
        const _: () = {
            #descriptor
            #listed
            #step_hooks
            #(#step_checks)*
            #workflow_hooks
            #workflow_checks
            #services
            #builder
        };
    })
}

/// The names of the steps the workflow lists, then those of their rescue steps, which the checks
/// of its input adapters and providers read.
fn listed(workflow: &Ident, steps: &[WithStep]) -> TokenStream {
    let rescues = steps.iter().filter_map(WithStep::rescue_name);
    let names = steps.iter().map(|step| step.name.value()).chain(rescues);
    quote! {
        impl #workflow {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub(crate) const __ITINERA_STEPS: &'static [&'static str] = &[#(#names),*];
        }
    }
}

/// What the workflow would have, as far as it can be read: a `Workflow` that is never called, the
/// names of the steps that could be read and of their rescue steps, and its instance builder,
/// with a setter for each field that could be read, all in the mode of the first step that could
/// be read. A generic workflow gets none, since none of them could name it.
fn stand_in(
    declared: &ItemStruct,
    marks: &[Attribute],
    fields: &[Settable],
    complete: bool,
) -> TokenStream {
    if !declared.generics.params.is_empty() {
        return TokenStream::new();
    }
    let workflow = &declared.ident;
    let steps: Vec<WithStep> = marks
        .iter()
        .map(WithStep::read)
        .filter_map(Result::ok)
        .collect();
    let chosen = mode::of(&steps);
    let flagged = mode::flagged();
    let mode = mode::named(workflow);
    let listed = listed(workflow, &steps);
    let builder = builder::builder(workflow, &declared.vis, fields, &mode, complete);
    quote! {
        const _: () = {
            impl ::itinera::workflow::Workflow for #workflow {
                type Mode = #chosen;

                fn descriptor() -> &'static ::itinera::workflow::WorkflowDescriptor<Self, Self::Mode> {
                    ::core::unreachable!()
                }
            }

            #flagged

            #listed
            #builder
        };
    }
}
