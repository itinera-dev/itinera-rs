//! A step's services: declaring them, taking those it was lent, and using them.

use itinera::error::Error;
use itinera::service::Service;

use super::building::{Builds, Needs};
use crate::declaration::leaked;
use crate::model::ServiceType;
use crate::service::{Labeller, Meter};

/// A service the step requests, as a token of its type.
#[derive(Debug)]
pub(super) enum Token {
    Labeller(Service<dyn Labeller>),
    Meter(Service<dyn Meter>),
}

impl Token {
    pub(super) fn of((key, service_type): &(String, ServiceType)) -> Self {
        let key = leaked(key);
        match service_type {
            ServiceType::Labeller => Self::Labeller(Service::new(key)),
            ServiceType::Meter => Self::Meter(Service::new(key)),
        }
    }

    /// The service lent to the attempt for this token.
    pub(super) fn lent<'a, G: Builds<'a>>(&self, got: &G) -> Result<Lent<'a>, Error> {
        Ok(match self {
            Self::Labeller(service) => Lent::Labeller(service.key(), got.service(service)?),
            Self::Meter(service) => Lent::Meter(service.key(), got.service(service)?),
        })
    }
}

/// Declares the service, with its type.
pub(super) fn needing<N: Needs>(needs: N, token: &Token) -> N {
    match token {
        Token::Labeller(service) => needs.service(service),
        Token::Meter(service) => needs.service(service),
    }
}

/// A service lent to an attempt, under its key.
#[derive(derive_more::Debug)]
pub(super) enum Lent<'a> {
    Labeller(&'static str, #[debug(skip)] &'a dyn Labeller),
    Meter(&'static str, #[debug(skip)] &'a dyn Meter),
}

impl Lent<'_> {
    pub(super) fn key(&self) -> &'static str {
        match self {
            Self::Labeller(key, _) | Self::Meter(key, _) => key,
        }
    }

    /// Calls the service's one operation, and tells what it returned as text: a labeller's
    /// label, or a meter's reading.
    pub(super) fn label(&self) -> Result<String, Error> {
        match self {
            Self::Labeller(_, labeller) => labeller.label(),
            Self::Meter(_, meter) => Ok(meter.reading()?.to_string()),
        }
    }
}
