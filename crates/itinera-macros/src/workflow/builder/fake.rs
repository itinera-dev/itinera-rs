//! The builder's `fake_provider`, which takes a fake in place of a service provider only when
//! itinera's `testing` feature implements the bound it requires.

use proc_macro2::TokenStream;
use quote::quote;

/// The builder's `fake_provider`, for a workflow whose instance builder is `instance_builder`.
pub(super) fn fake_provider(instance_builder: &TokenStream) -> TokenStream {
    quote! {
        #[doc = "Puts `fake` in place of the function of the workflow's service provider named `name`, for this instance alone, with the `testing` feature: the provider keeps its scope, its steps and its requests, and `fake` makes each instance it would make. Name the service's type, as in `fake_provider::<dyn PaymentGateway>`. `create()` panics when the workflow has no provider of that name, or when the type is not the one the provider supplies."]
        pub fn fake_provider<__ItineraService: ?::core::marker::Sized + 'static>(
            mut self,
            name: impl ::core::convert::Into<::itinera::service::ProviderName>,
            fake: impl ::core::ops::Fn() -> ::core::result::Result<
                ::itinera::service::Provided<__ItineraService>,
                ::itinera::error::Error,
            > + ::core::marker::Send + ::core::marker::Sync + 'static,
        ) -> Self
        where
            #instance_builder: ::itinera::instance::FakeProvider<__ItineraService>,
        {
            let name = name.into();
            self.calls.push(::std::boxed::Box::new(move |instance: #instance_builder| {
                ::itinera::instance::FakeProvider::__itinera_fake_provider(instance, name, fake)
            }));
            self
        }
    }
}
