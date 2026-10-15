//! The scopes of a service provider: how long an instance it makes is lent.

/// A scope of a service provider, part of its type: [`PerJourney`] or [`PerAttempt`]. It decides
/// what the provider may request, since only a provider called for each attempt knows which step
/// and attempt it serves.
///
/// No other type can be a scope.
///
/// # Examples
///
/// ```
/// use itinera::policy::{HookNeeds, Provider};
/// use itinera::service::ProviderScope;
///
/// fn reporting<S: ProviderScope>() -> HookNeeds<Provider<S>> {
///     HookNeeds::new().reporter()
/// }
/// ```
pub trait ProviderScope: sealed::Sealed + Copy + Send + Sync + 'static {}

/// The scope of a provider called once per journey, the first time a step it serves needs its
/// service: every later attempt of every step it serves receives that same instance, until the
/// journey ends.
///
/// # Examples
///
/// ```
/// use itinera::policy::{Provider, Requested};
/// use itinera::service::PerJourney;
///
/// struct Orders;
///
/// fn journey_id(got: &Requested<'_, Orders, Provider<PerJourney>>) -> String {
///     got.journey_id().to_string()
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub enum PerJourney {}

/// The scope of a provider called for every attempt of every step it serves: each instance
/// serves one attempt only.
///
/// # Examples
///
/// ```
/// use itinera::policy::{Provider, Requested};
/// use itinera::service::PerAttempt;
/// use itinera::step::Attempt;
///
/// struct Orders;
///
/// fn attempt(got: &Requested<'_, Orders, Provider<PerAttempt>>) -> Attempt {
///     got.attempt()
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub enum PerAttempt {}

impl ProviderScope for PerJourney {}
impl ProviderScope for PerAttempt {}

pub(crate) use sealed::Scope;

mod sealed {
    use super::{PerAttempt, PerJourney};

    pub trait Sealed {
        /// The scope, as the executor keeps track of the instances a provider of it made.
        const SCOPE: Scope;
    }

    /// A provider's scope, as the executor keeps track of the instances it made.
    ///
    /// It is public in this private module, since `Sealed`, which `ProviderScope` exposes, names
    /// it; nothing outside the crate can name it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Scope {
        /// An instance is kept until the journey ends.
        Journey,
        /// An instance serves one attempt only.
        Attempt,
    }

    impl Sealed for PerJourney {
        const SCOPE: Scope = Scope::Journey;
    }

    impl Sealed for PerAttempt {
        const SCOPE: Scope = Scope::Attempt;
    }
}
