//! The level of an event a step or hook emits about its own work.

/// The level of an event a step or hook emits about its own work: `step_info`, `step_warning`
/// or `step_error` for a step, and `journey_info`, `journey_warning` or `journey_error` for a
/// hook.
///
/// # Examples
///
/// ```
/// use itinera::event::Level;
///
/// fn worth_a_look(level: Level) -> bool {
///     matches!(level, Level::Warning | Level::Error)
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// Information: `step_info` or `journey_info`.
    Info,
    /// A warning: `step_warning` or `journey_warning`.
    Warning,
    /// An error, which changes no outcome: `step_error` or `journey_error`.
    Error,
}
