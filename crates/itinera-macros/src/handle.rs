//! The handles a parameter or field takes, which are recognised by their type rather than marked.

use syn::{Type, TypeGroup, TypeParen, TypePath, TypeReference};

use crate::datum::option_of;

/// A handle, or something else itinera gives, that a parameter or field takes, as its type says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handle {
    /// `&mut Contributor`.
    Contributor,
    /// A reporter, such as `StepReporter<'_>` or `HookReporter<'_>`.
    Reporter,
    /// `&JourneyId`.
    JourneyId,
    /// `StepName`.
    StepName,
    /// `Attempt`.
    Attempt,
    /// `Reason`, or `Option<Reason>` when it is optional.
    Reason { optional: bool },
    /// `&Error` for a hook, or `Error` for a rescue step, or either in an `Option` when it is
    /// optional.
    Error { optional: bool },
    /// `StepFailureCause` or `RetryCause`.
    Cause,
    /// `&DataBag`.
    DataBag,
}

impl Handle {
    /// The handle a parameter or field of this type takes, if its type is one of theirs, by the
    /// last segment of the type's path.
    pub(crate) fn of(ty: &Type) -> Option<Self> {
        let ty = bare(ty);
        match option_of(ty) {
            Some(held) => match written(held)? {
                Self::Reason { .. } => Some(Self::Reason { optional: true }),
                Self::Error { .. } => Some(Self::Error { optional: true }),
                _ => None,
            },
            None => written(ty),
        }
    }
}

/// The handle a type not wrapped in an `Option` takes, if any.
fn written(ty: &Type) -> Option<Handle> {
    match bare(ty) {
        Type::Reference(TypeReference {
            mutability: Some(_),
            elem,
            ..
        }) => match named(elem)?.as_str() {
            "Contributor" => Some(Handle::Contributor),
            _ => None,
        },
        Type::Reference(TypeReference { elem, .. }) => match named(elem)?.as_str() {
            "JourneyId" => Some(Handle::JourneyId),
            "Error" => Some(Handle::Error { optional: false }),
            "DataBag" => Some(Handle::DataBag),
            _ => None,
        },
        _ => match named(ty)?.as_str() {
            "StepName" => Some(Handle::StepName),
            "Attempt" => Some(Handle::Attempt),
            "Reason" => Some(Handle::Reason { optional: false }),
            "Error" => Some(Handle::Error { optional: false }),
            "StepFailureCause" | "RetryCause" => Some(Handle::Cause),
            "StepReporter" | "AsyncStepReporter" | "HookReporter" | "AsyncHookReporter" => {
                Some(Handle::Reporter)
            }
            _ => None,
        },
    }
}

/// The type inside any parentheses, or the invisible group a `macro_rules!` macro wraps a type
/// in.
fn bare(ty: &Type) -> &Type {
    match ty {
        Type::Group(TypeGroup { elem, .. }) | Type::Paren(TypeParen { elem, .. }) => bare(elem),
        _ => ty,
    }
}

/// The name of the type, the last segment of its path, without its arguments.
fn named(ty: &Type) -> Option<String> {
    let Type::Path(TypePath {
        qself: None, path, ..
    }) = bare(ty)
    else {
        return None;
    };
    let name = &path.segments.last()?.ident;
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn handle(ty: &str) -> Option<Handle> {
        Handle::of(&syn::parse_str(ty).unwrap())
    }

    #[rstest]
    #[case::a_contributor("&mut Contributor", Handle::Contributor)]
    #[case::a_contributor_by_its_path("&mut itinera::journey::Contributor", Handle::Contributor)]
    #[case::a_step_reporter("StepReporter<'_>", Handle::Reporter)]
    #[case::an_asynchronous_hook_reporter("AsyncHookReporter<'_>", Handle::Reporter)]
    #[case::the_journey_id("&JourneyId", Handle::JourneyId)]
    #[case::the_step_name("StepName", Handle::StepName)]
    #[case::the_attempt("Attempt", Handle::Attempt)]
    #[case::the_reason("Reason", Handle::Reason { optional: false })]
    #[case::an_optional_reason("Option<Reason>", Handle::Reason { optional: true })]
    #[case::the_error("&Error", Handle::Error { optional: false })]
    #[case::an_optional_error("Option<&Error>", Handle::Error { optional: true })]
    #[case::the_error_by_value("Error", Handle::Error { optional: false })]
    #[case::an_optional_error_by_value("Option<Error>", Handle::Error { optional: true })]
    #[case::the_cause_of_a_retry("RetryCause", Handle::Cause)]
    #[case::the_cause_of_a_failure("StepFailureCause", Handle::Cause)]
    #[case::the_data_bag("&DataBag", Handle::DataBag)]
    #[case::the_attempt_in_parentheses("(Attempt)", Handle::Attempt)]
    #[case::a_contributor_in_parentheses("&mut (Contributor)", Handle::Contributor)]
    #[case::an_optional_reason_in_parentheses("(Option<(Reason)>)", Handle::Reason { optional: true })]
    fn a_handle_is_recognised_by_its_type(#[case] ty: &str, #[case] expected: Handle) {
        assert_eq!(handle(ty), Some(expected));
    }

    #[test]
    fn a_handle_is_recognised_in_the_group_a_macro_by_example_wraps_it_in() {
        let grouped = Type::Group(TypeGroup {
            attrs: Vec::new(),
            group_token: syn::token::Group::default(),
            elem: Box::new(syn::parse_str("Attempt").unwrap()),
        });
        assert_eq!(Handle::of(&grouped), Some(Handle::Attempt));
    }

    #[rstest]
    #[case::a_value("i64")]
    #[case::a_service("&dyn Gateway")]
    #[case::a_contributor_shared("&Contributor")]
    #[case::a_journey_id_by_value("JourneyId")]
    #[case::an_optional_attempt("Option<Attempt>")]
    fn a_type_that_is_no_handle_is_not_recognised(#[case] ty: &str) {
        assert_eq!(handle(ty), None);
    }
}
