//! Roles: the operations a workflow's own type provides to the hooks of its policies.

/// A role the workflow's own type provides: the operations of the trait `R`, which policies
/// reach through [`Requested::role`].
///
/// A workflow implements it once for each role, returning itself. A policy whose hooks request a
/// role is implemented only for workflows that provide it, so attaching it to a workflow without
/// the role does not compile.
///
/// [`Requested::role`]: super::Requested::role
///
/// # Examples
///
/// ```
/// use itinera::error::Error;
/// use itinera::policy::Provides;
///
/// trait Notifier {
///     fn notify(&self, message: &str) -> Result<(), Error>;
/// }
///
/// struct Orders;
///
/// impl Notifier for Orders {
///     fn notify(&self, _message: &str) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// impl Provides<dyn Notifier> for Orders {
///     fn role(&self) -> &(dyn Notifier + 'static) {
///         self
///     }
/// }
///
/// Orders.role().notify("order shipped")?;
/// # Ok::<(), Error>(())
/// ```
#[diagnostic::on_unimplemented(
    message = "the workflow `{Self}` does not provide the role `{R}`",
    label = "this workflow does not provide the role",
    note = "a workflow provides a role by implementing `Provides<{R}>`"
)]
pub trait Provides<R: ?Sized> {
    /// The workflow, as the role.
    ///
    /// # Examples
    ///
    /// ```
    /// use itinera::policy::Provides;
    ///
    /// trait Clock {
    ///     fn hour(&self) -> u8;
    /// }
    ///
    /// struct Orders;
    ///
    /// impl Clock for Orders {
    ///     fn hour(&self) -> u8 {
    ///         9
    ///     }
    /// }
    ///
    /// impl Provides<dyn Clock> for Orders {
    ///     fn role(&self) -> &(dyn Clock + 'static) {
    ///         self
    ///     }
    /// }
    ///
    /// let clock: &dyn Clock = Orders.role();
    /// assert_eq!(clock.hour(), 9);
    /// ```
    fn role(&self) -> &R;
}
