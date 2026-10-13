//! The macro that makes what a service provider returns from its instance.

/// Makes the [`Provided`] a service provider returns from an instance, reading the name of the
/// instance's concrete type before the instance becomes the service's type, such as
/// `dyn PaymentGateway`. Written `provide!(instance)`, or `provide!(instance, data)` to supply
/// data with it, which must be a [`Value`] and which `service_provided` carries for the reporters.
///
/// It is written where the service's type is known, such as in `Ok(provide!(Sandbox))` returned by
/// a provider: Rust turns a box of the instance into a box of a trait object only there. A
/// `Provided` bound by `let` names its type, as in `let made: Provided<dyn PaymentGateway>`.
///
/// [`Provided`]: crate::service::Provided
/// [`Value`]: crate::value::Value
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::service::{Provided, provide};
///
/// trait PaymentGateway: Send + Sync {}
///
/// struct Bank;
/// struct Sandbox;
///
/// impl PaymentGateway for Bank {}
/// impl PaymentGateway for Sandbox {}
///
/// fn payments(testing: bool) -> Result<Provided<dyn PaymentGateway>, Error> {
///     if testing {
///         Ok(provide!(Sandbox, "sandbox".to_string()))
///     } else {
///         Ok(provide!(Bank))
///     }
/// }
///
/// let made = payments(true)?;
/// assert!(made.implementation().to_string().ends_with("Sandbox"));
/// assert!(made.data().is_some());
/// # Ok::<(), Error>(())
/// ```
///
/// Data that is not a value does not compile:
///
/// ```compile_fail,E0277
/// use itinera::service::{Provided, provide};
///
/// trait PaymentGateway: Send + Sync {}
///
/// struct Bank;
///
/// impl PaymentGateway for Bank {}
///
/// let made: Provided<dyn PaymentGateway> = provide!(Bank, std::sync::Mutex::new(1));
/// ```
#[doc(hidden)]
#[macro_export]
macro_rules! __provide {
    ($service:expr $(,)?) => {
        $crate::__provide!(@made $service, ::core::option::Option::None)
    };
    ($service:expr, $data:expr $(,)?) => {
        $crate::__provide!(
            @made $service,
            ::core::option::Option::Some($crate::value::AnyValue::new($data))
        )
    };
    (@made $service:expr, $data:expr) => {{
        let service = $service;
        $crate::service::Provided::__new(
            $crate::service::Implementation::__of(&service),
            ::std::boxed::Box::new(service),
            $data,
        )
    }};
}

#[doc(inline)]
pub use crate::__provide as provide;
