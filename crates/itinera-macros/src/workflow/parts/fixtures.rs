//! Fixtures that the tests of several parts share: providers read from methods written as text.

use syn::ImplItemFn;

use super::provider::Provider;

/// The provider a method of a workflow's block describes, or why it describes none.
pub(super) fn read(method: &str) -> syn::Result<Provider> {
    let mut function: ImplItemFn = syn::parse_str(method)?;
    let mark = function.attrs.remove(0);
    Provider::read(&mut function, &mark)
}

/// The provider a method of a workflow's block describes, which must describe one.
pub(super) fn provider(method: &str) -> Provider {
    read(method).unwrap()
}
