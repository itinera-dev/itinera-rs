//! A service a step requests, from a parameter of its `run` or `new` marked `#[service]`.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Attribute, Ident, LitStr, Meta, Pat, PatType, Type};

use crate::described;
use crate::service_key::key_type;

/// A service a step requests: its key, the type it is lent as, and where the parameter's type is
/// written, so that a service of a type that cannot be one is reported there.
pub(super) struct Service {
    key: LitStr,
    service: Type,
    ty: Span,
}

impl Service {
    /// Reads the service a parameter marked `#[service]` requests, named after the parameter
    /// unless the mark names its key, as in `#[service("payments")]`.
    pub(super) fn read(mark: &Attribute, parameter: &PatType) -> syn::Result<Self> {
        let key = match &mark.meta {
            Meta::Path(_) => named(&parameter.pat)?,
            _ => mark.parse_args::<LitStr>().map_err(misread)?,
        };
        let Type::Reference(reference) = &*parameter.ty else {
            return Err(not_borrowed(&parameter.ty));
        };
        if reference.mutability.is_some() {
            return Err(not_borrowed(&parameter.ty));
        }
        Ok(Self {
            key,
            service: (*reference.elem).clone(),
            ty: parameter.ty.span(),
        })
    }

    pub(super) fn key(&self) -> &LitStr {
        &self.key
    }

    pub(super) fn shares_key_with(&self, other: &Self) -> bool {
        self.key.value() == other.key.value()
    }

    /// The token naming the service, such as `Service::<dyn PaymentGateway>::new("payments")`.
    fn token(&self) -> TokenStream {
        let Self { key, service, .. } = self;
        quote_spanned!(self.ty=> ::itinera::service::Service::<#service>::new(#key))
    }

    /// The call that declares the service on the step's needs.
    pub(super) fn declared(&self) -> TokenStream {
        let token = self.token();
        quote_spanned!(self.ty=> .service(&#token))
    }

    /// The expression that takes the service from what was resolved, lent for the attempt.
    pub(super) fn taken(&self, got: &Ident) -> TokenStream {
        let token = self.token();
        quote_spanned!(self.ty=> #got.service(&#token)?)
    }

    /// The type of the attempt's field holding the service, borrowed for the attempt.
    pub(super) fn borrowed(&self) -> TokenStream {
        let service = &self.service;
        quote!(&'a #service)
    }

    /// The bound on a workflow that its providers of the service's key make the type the step
    /// requests, reported at the parameter's type.
    fn fits(&self, workflow: &Ident) -> TokenStream {
        let key = key_type(&self.key);
        let service = &self.service;
        quote_spanned! {self.ty=>
            #workflow: ::itinera::service::ProvidesService<#key, #service>,
        }
    }

    /// The check, run as the program compiles, that another parameter requesting the same service
    /// requests it as the same type, reported at this one.
    pub(super) fn same_type_as(&self, other: &Self) -> TokenStream {
        let (service, other) = (&self.service, &other.service);
        quote_spanned! {self.ty=>
            const _: () = {
                let _: ::core::marker::PhantomData<#other> = ::core::marker::PhantomData::<#service>;
            };
        }
    }
}

/// The key a service is named after: its parameter's name.
fn named(pattern: &Pat) -> syn::Result<LitStr> {
    match pattern {
        Pat::Ident(pattern) => Ok(described::named_after(&pattern.ident)),
        _ => Err(syn::Error::new_spanned(
            pattern,
            "name the service's key, as in `#[service(\"payments\")]`, since this parameter has \
             no name to take it from",
        )),
    }
}

fn misread(error: syn::Error) -> syn::Error {
    syn::Error::new(
        error.span(),
        "`#[service]` takes the service's key, such as `#[service(\"payments\")]`",
    )
}

fn not_borrowed(ty: &Type) -> syn::Error {
    syn::Error::new_spanned(
        ty,
        "a service is lent to the step, taken by shared reference, such as \
         `&dyn PaymentGateway`",
    )
}

/// The step's hidden function whose bounds say that a workflow's providers make each of these
/// services as the type the step requests. The workflow's struct names it for each step it lists,
/// so that a provider of another type does not compile.
pub(super) fn fitted(step: &Type, services: &[&Service]) -> Option<TokenStream> {
    if services.is_empty() {
        return None;
    }
    let workflow = format_ident!("__ItineraWorkflow");
    let bounds = services.iter().map(|service| service.fits(&workflow));
    // Public, so that a workflow can check a step of another crate; a public step may still
    // request a service through a trait private to its own crate.
    Some(quote! {
        impl #step {
            #[doc(hidden)]
            #[allow(private_bounds)]
            pub fn __itinera_services_fit<#workflow>()
            where
                #(#bounds)*
            {
            }
        }
    })
}

/// The first service that an earlier one in the list requests already.
pub(super) fn repeated(services: &[&Service]) -> Option<syn::Error> {
    services
        .iter()
        .enumerate()
        .find_map(|(index, service)| requested_before(services, index, service))
}

fn requested_before(services: &[&Service], index: usize, service: &Service) -> Option<syn::Error> {
    services
        .iter()
        .take(index)
        .any(|earlier| earlier.shares_key_with(service))
        .then(|| twice(service))
}

fn twice(service: &Service) -> syn::Error {
    syn::Error::new_spanned(
        &service.key,
        format!(
            "service requested twice: {}, which another parameter of this function requests \
             already",
            service.key.value()
        ),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use syn::{FnArg, ItemFn};

    use super::*;

    /// The service the only parameter of this function requests.
    fn read(function: &str) -> syn::Result<Service> {
        let function: ItemFn = syn::parse_str(function)?;
        let Some(FnArg::Typed(parameter)) = function.sig.inputs.first() else {
            panic!("the function has one typed parameter");
        };
        Service::read(&parameter.attrs[0], parameter)
    }

    /// The code without its spaces, which tokens place differently from the way code is written.
    fn compact(code: &str) -> String {
        code.split_whitespace().collect()
    }

    #[rstest]
    #[case::named_after_its_parameter(
        "fn run(#[service] stock: &dyn StockStore) {}",
        r#".service(&::itinera::service::Service::<dyn StockStore>::new("stock"))"#
    )]
    #[case::with_its_own_key(
        r#"fn run(#[service("payments")] gateway: &dyn PaymentGateway) {}"#,
        r#".service(&::itinera::service::Service::<dyn PaymentGateway>::new("payments"))"#
    )]
    #[case::named_after_a_raw_identifier(
        "fn run(#[service] r#box: &Warehouse) {}",
        r#".service(&::itinera::service::Service::<Warehouse>::new("box"))"#
    )]
    #[case::borrowed_for_a_named_lifetime(
        "fn run(#[service] stock: &'a dyn StockStore) {}",
        r#".service(&::itinera::service::Service::<dyn StockStore>::new("stock"))"#
    )]
    fn a_service_is_declared_by_its_key_and_the_type_it_is_lent_as(
        #[case] function: &str,
        #[case] expected: &str,
    ) {
        let declared = read(function).unwrap().declared().to_string();
        assert_eq!(compact(&declared), compact(expected));
    }

    #[rstest]
    #[case::taken_by_value(
        "fn run(#[service] stock: Box<dyn StockStore>) {}",
        "a service is lent to the step, taken by shared reference, such as `&dyn PaymentGateway`"
    )]
    #[case::taken_by_mutable_reference(
        "fn run(#[service] stock: &mut dyn StockStore) {}",
        "a service is lent to the step, taken by shared reference, such as `&dyn PaymentGateway`"
    )]
    #[case::named_by_something_other_than_a_string(
        "fn run(#[service(stock)] stock: &dyn StockStore) {}",
        r#"`#[service]` takes the service's key, such as `#[service("payments")]`"#
    )]
    #[case::of_a_parameter_without_a_name(
        "fn run(#[service] (stock, _): &dyn StockStore) {}",
        r#"name the service's key, as in `#[service("payments")]`, since this parameter has no name to take it from"#
    )]
    fn a_service_is_refused_when_its_parameter_cannot_request_it(
        #[case] function: &str,
        #[case] refused: &str,
    ) {
        let error = read(function).err().map(|error| error.to_string());
        assert_eq!(error.as_deref(), Some(refused));
    }

    #[test]
    fn a_service_requested_by_two_parameters_of_one_function_is_refused_at_the_second() {
        let first = read("fn run(#[service] payments: &dyn PaymentGateway) {}").unwrap();
        let second = read(r#"fn run(#[service("payments")] backup: &dyn Backup) {}"#).unwrap();

        let error = repeated(&[&first, &second]).map(|error| error.to_string());

        assert_eq!(
            error.as_deref(),
            Some(
                "service requested twice: payments, which another parameter of this function \
                 requests already"
            )
        );
    }
}
