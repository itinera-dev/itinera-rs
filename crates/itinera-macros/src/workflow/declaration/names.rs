//! The checks on the names of the steps a workflow lists, counting their rescue steps' names.

use super::super::repeated;
use super::super::with_step::WithStep;

/// Refuses a step name that an earlier `#[with_step]` line already gave, or that is the name of
/// a rescue step.
pub(super) fn refuse_duplicates(steps: &[WithStep]) -> syn::Result<()> {
    if let Some(step) = repeated(steps, WithStep::shares_name_with) {
        return Err(syn::Error::new_spanned(
            &step.name,
            format!(
                "duplicate step name: {}, which an earlier `#[with_step]` line already gives",
                step.name.value()
            ),
        ));
    }
    let rescues: Vec<String> = steps.iter().filter_map(WithStep::rescue_name).collect();
    match steps.iter().find(|step| step.is_named_among(&rescues)) {
        Some(step) => Err(syn::Error::new_spanned(
            &step.name,
            format!(
                "duplicate step name: {}, which is the name of a rescue step",
                step.name.value()
            ),
        )),
        None => Ok(()),
    }
}
