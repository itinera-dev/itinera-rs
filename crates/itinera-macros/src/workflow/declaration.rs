//! `#[workflow(...)]` on a struct: the workflow's descriptor, held in a hidden static, and its
//! instance builder.

mod descriptor;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Ident, ItemStruct};

use super::arguments::Arguments;
use super::builder::{self, Settable};
use super::checks::{self, Attached};
use super::repeated;
use super::with_step::WithStep;
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
        None => declare(arguments.clone(), &declared, &marks, &fields),
        Some(error) => Err(error),
    };
    match declaration {
        Ok(declaration) => quote! {
            #declared
            #declaration
        },
        Err(error) => {
            let error = error.into_compile_error();
            let stand_in = stand_in(arguments, &declared, &marks, &fields, complete);
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
    refuse_duplicate_names(&steps)?;
    let mode = described::mode(arguments.is_asynchronous());
    let name = arguments
        .name
        .clone()
        .unwrap_or_else(|| described::named_after(workflow));
    let descriptor = descriptor::descriptor(workflow, &name, &arguments, &steps, &mode);
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
    let builder = builder::builder(
        workflow,
        &declared.vis,
        fields,
        &mode,
        arguments.is_asynchronous(),
        true,
    );
    Ok(quote! {
        const _: () = {
            #descriptor
            #listed
            #step_hooks
            #(#step_checks)*
            #workflow_hooks
            #workflow_checks
            #builder
        };
    })
}

/// Refuses a step name that an earlier `#[with_step]` line already gave.
fn refuse_duplicate_names(steps: &[WithStep]) -> syn::Result<()> {
    match repeated(steps, WithStep::shares_name_with) {
        Some(step) => Err(syn::Error::new_spanned(
            &step.name,
            format!(
                "duplicate step name: {}, which an earlier `#[with_step]` line already gives",
                step.name.value()
            ),
        )),
        None => Ok(()),
    }
}

/// The names of the steps the workflow lists, which the checks of its input adapters read.
fn listed(workflow: &Ident, steps: &[WithStep]) -> TokenStream {
    let names = steps.iter().map(|step| &step.name);
    quote! {
        impl #workflow {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub(crate) const __ITINERA_STEPS: &'static [&'static str] = &[#(#names),*];
        }
    }
}

/// What the workflow would have, as far as it can be read: a `Workflow` that is never called, the
/// names of the steps that could be read, and its instance builder, with a setter for each field
/// that could be read. A generic workflow gets none, since none of them could name it.
fn stand_in(
    arguments: TokenStream,
    declared: &ItemStruct,
    marks: &[Attribute],
    fields: &[Settable],
    complete: bool,
) -> TokenStream {
    if !declared.generics.params.is_empty() {
        return TokenStream::new();
    }
    let workflow = &declared.ident;
    let asynchronous =
        Arguments::read(arguments).is_ok_and(|arguments| arguments.is_asynchronous());
    let mode = described::mode(asynchronous);
    let steps: Vec<WithStep> = marks
        .iter()
        .map(WithStep::read)
        .filter_map(Result::ok)
        .collect();
    let listed = listed(workflow, &steps);
    let builder = builder::builder(
        workflow,
        &declared.vis,
        fields,
        &mode,
        asynchronous,
        complete,
    );
    quote! {
        const _: () = {
            impl ::itinera::workflow::Workflow for #workflow {
                type Mode = #mode;

                fn descriptor() -> &'static ::itinera::workflow::WorkflowDescriptor<Self, #mode> {
                    ::core::unreachable!()
                }
            }

            #listed
            #builder
        };
    }
}
