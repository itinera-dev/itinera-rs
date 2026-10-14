#[cfg(feature = "async")]
use crate::mode::Asynchronous;
use crate::mode::Synchronous;
#[cfg(feature = "async")]
use crate::report::AsyncDispatcherFactory;
use crate::report::DispatcherFactory;

pub trait Sealed<M> {}

impl<F: DispatcherFactory> Sealed<Synchronous> for F {}

#[cfg(feature = "async")]
impl<F: AsyncDispatcherFactory> Sealed<Asynchronous> for F {}
