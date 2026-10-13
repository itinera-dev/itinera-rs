//! The macros are syntax over the builder: what each one declares is what the builder declares
//! when the same parts are written by hand, and it runs the same way on both executors.

#![cfg(feature = "macros")]

// Marked as test code, where the workspace's lints allow `unwrap`.
#[cfg(test)]
#[cfg(feature = "async")]
mod async_policy;
#[cfg(test)]
#[cfg(feature = "async")]
mod async_recorders;
#[cfg(test)]
#[cfg(feature = "async")]
mod async_rescue;
#[cfg(test)]
#[cfg(feature = "async")]
mod async_service;
#[cfg(test)]
#[cfg(feature = "async")]
mod async_step;
#[cfg(test)]
#[cfg(feature = "async")]
mod async_workflow;
#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod journey;
#[cfg(test)]
mod policy;
#[cfg(test)]
mod recorders;
#[cfg(test)]
mod rescue;
#[cfg(test)]
mod rescue_service;
#[cfg(test)]
mod service;
#[cfg(test)]
mod step;
#[cfg(test)]
mod workflow;
