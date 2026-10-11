//! The `impl` block an attribute macro is written on: its functions, their parameters, and the
//! shorthands the macro brings into scope in them.

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens as _;
use syn::{FnArg, ImplItem, ImplItemFn, ItemImpl, PatType, PathSegment, ReturnType, Stmt, Type};

use crate::marker;

/// The functions of the block.
pub(crate) fn functions(block: &mut ItemImpl) -> impl Iterator<Item = &mut ImplItemFn> {
    block.items.iter_mut().filter_map(function)
}

fn function(item: &mut ImplItem) -> Option<&mut ImplItemFn> {
    match item {
        ImplItem::Fn(function) => Some(function),
        _ => None,
    }
}

/// The parameter, unless it is the receiver.
pub(crate) fn typed(argument: &mut FnArg) -> Option<&mut PatType> {
    match argument {
        FnArg::Typed(parameter) => Some(parameter),
        FnArg::Receiver(_) => None,
    }
}

/// Removes these marks from the function and its parameters, where an error left them.
pub(crate) fn unmark(function: &mut ImplItemFn, marks: &[&str]) {
    marker::take(&mut function.attrs, marks);
    for parameter in function.sig.inputs.iter_mut().filter_map(typed) {
        marker::take(&mut parameter.attrs, marks);
    }
}

/// Starts the function's body with `import` when the body writes one of these shorthands, such
/// as `Self::succeed`, so that it finds them.
pub(crate) fn bring_into_scope(
    function: &mut ImplItemFn,
    import: &TokenStream,
    shorthands: &[&str],
) {
    if writes_any(function.block.to_token_stream(), shorthands) {
        function
            .block
            .stmts
            .insert(0, Stmt::Item(syn::Item::Verbatim(import.clone())));
    }
}

/// Whether the tokens write `Self::` followed by one of these names, at any depth.
fn writes_any(tokens: TokenStream, names: &[&str]) -> bool {
    let tokens: Vec<TokenTree> = tokens.into_iter().collect();
    tokens.windows(4).any(|window| is_written(window, names))
        || tokens.into_iter().any(|token| holds_any(token, names))
}

fn is_written(window: &[TokenTree], names: &[&str]) -> bool {
    match window {
        [
            TokenTree::Ident(this),
            TokenTree::Punct(first),
            TokenTree::Punct(second),
            TokenTree::Ident(name),
        ] => {
            this == "Self"
                && first.as_char() == ':'
                && second.as_char() == ':'
                && names.contains(&name.to_string().as_str())
        }
        _ => false,
    }
}

fn holds_any(token: TokenTree, names: &[&str]) -> bool {
    match token {
        TokenTree::Group(group) => writes_any(group.stream(), names),
        _ => false,
    }
}

/// Whether the function returns a `Result`, written as one.
pub(crate) fn returns_result(output: &ReturnType) -> bool {
    let ReturnType::Type(_, ty) = output else {
        return false;
    };
    let Type::Path(path) = &**ty else {
        return false;
    };
    path.path.segments.last().is_some_and(is_result)
}

fn is_result(segment: &PathSegment) -> bool {
    segment.ident == "Result"
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::called_directly(quote!({ Self::succeed() }), true)]
    #[case::called_in_a_closure(quote!({ let done = || Self::succeed(); done() }), true)]
    #[case::called_in_a_nested_block(quote!({ if late { { Self::fail("late") } } }), true)]
    #[case::another_function_of_the_type(quote!({ Self::new() }), false)]
    #[case::none(quote!({ Ok(Outcome::success()) }), false)]
    fn a_shorthand_is_found_wherever_the_body_writes_it(
        #[case] body: TokenStream,
        #[case] written: bool,
    ) {
        assert_eq!(writes_any(body, &["succeed", "fail"]), written);
    }
}
