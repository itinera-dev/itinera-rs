//! Attributes that mark parameters, fields and functions for a macro, which the macro reads.

use syn::Attribute;

/// The marks among these attributes that this macro reads, removed from them, in order.
pub(crate) fn take(attrs: &mut Vec<Attribute>, names: &[&str]) -> Vec<Attribute> {
    let (marks, others) = std::mem::take(attrs)
        .into_iter()
        .partition(|attr| is_one_of(attr, names));
    *attrs = others;
    marks
}

/// The marks among these attributes that this macro reads, in order, left in place.
pub(crate) fn found<'a>(attrs: &'a [Attribute], names: &[&str]) -> Vec<&'a Attribute> {
    attrs.iter().filter(|attr| is_one_of(attr, names)).collect()
}

fn is_one_of(attr: &Attribute, names: &[&str]) -> bool {
    names.contains(&name(attr).as_str())
}

/// The one mark an item carries, such as a parameter or a field, or an error pointing at the item
/// when it carries none, or at its second mark.
pub(crate) fn single<A: quote::ToTokens>(
    marks: Vec<A>,
    item: &impl quote::ToTokens,
    missing: &str,
) -> syn::Result<A> {
    let mut marks = marks.into_iter();
    match (marks.next(), marks.next()) {
        (Some(mark), None) => Ok(mark),
        (None, _) => Err(syn::Error::new_spanned(item, missing)),
        (Some(_), Some(second)) => Err(syn::Error::new_spanned(
            second,
            "this attribute is one too many: each item carries one of them",
        )),
    }
}

/// The one mark an item carries, if it carries one, or an error pointing at its second mark.
pub(crate) fn at_most_one<A: quote::ToTokens>(
    marks: Vec<A>,
    too_many: &str,
) -> syn::Result<Option<A>> {
    let mut marks = marks.into_iter();
    match (marks.next(), marks.next()) {
        (mark, None) => Ok(mark),
        (_, Some(second)) => Err(syn::Error::new_spanned(second, too_many)),
    }
}

/// The name of a mark, such as `input` for `#[input("amount")]`.
pub(crate) fn name(mark: &Attribute) -> String {
    mark.path()
        .get_ident()
        .map(ToString::to_string)
        .unwrap_or_default()
}
