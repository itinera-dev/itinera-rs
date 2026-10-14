//! The workflows that the `@macros` features drive, declared only with itinera's macros, each
//! named by a scenario instead of a workflow it declares sentence by sentence.

mod courier;
mod delivery;
mod payment;
mod warehouse;

use std::collections::BTreeMap;

use itinera::instance::{Instance, WorkflowInstance as _};
use itinera::journey::InitialData;
use itinera::mode::{Asynchronous, Mode, Synchronous};
use itinera::value::AnyValue;
use itinera::workflow::{ListedStep, Workflow};

use crate::journey::{Journey, Ran, Runs, Unrunnable};
use crate::model::{Model, ModelError};
use crate::value::Typed;
use crate::world::World;
use courier::Couriers;
use delivery::Delivery;
use payment::Payment;
use warehouse::Warehouse;

/// A workflow the macros declare, by the name the scenarios give it.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    cucumber::Parameter,
    derive_more::Display,
    derive_more::FromStr,
)]
#[param(name = "fixture", regex = "payment|delivery|warehouse|courier")]
#[display(rename_all = "lowercase")]
#[from_str(rename_all = "lowercase")]
pub(crate) enum Fixture {
    /// Charges an order, then ships it; synchronous.
    Payment,
    /// Dispatches a parcel; asynchronous.
    Delivery,
    /// Reserves, charges and ships an order with the services its providers supply;
    /// synchronous.
    Warehouse,
    /// Packs and dispatches a parcel with the services its providers supply; asynchronous.
    Courier,
}

impl Fixture {
    /// The steps the workflow lists, as its descriptor gives them.
    pub(crate) fn listing(self) -> Vec<ListedStep> {
        match self {
            Self::Payment => Payment::descriptor().listing(),
            Self::Delivery => Delivery::descriptor().listing(),
            Self::Warehouse => Warehouse::descriptor().listing(),
            Self::Courier => Couriers::descriptor().listing(),
        }
    }
}

/// The fixture the scenario names, if it names one, which is then its only workflow.
pub(crate) fn named(world: &World) -> Result<Option<Fixture>, ModelError> {
    match (world.fixture, world.model.workflow()) {
        (Some(_), Ok(_)) => Err(ModelError::WorkflowDeclaredTwice),
        (fixture, _) => Ok(fixture),
    }
}

/// An execution mode, whose executor runs the fixtures declared in it.
pub(crate) trait RunsFixtures: Mode + Sized {
    /// Runs one journey of a new instance of the fixture, with the scenario's initial data.
    async fn journey<E: Runs<Self>>(
        fixture: Fixture,
        model: &Model,
        executor: &mut E,
    ) -> Result<Journey, Unrunnable>;
}

impl RunsFixtures for Synchronous {
    async fn journey<E: Runs<Self>>(
        fixture: Fixture,
        model: &Model,
        executor: &mut E,
    ) -> Result<Journey, Unrunnable> {
        let data = Entries::of(model)?;
        match fixture {
            Fixture::Payment => ran(payment::instance(data)?, executor).await,
            Fixture::Warehouse => ran(warehouse::instance(data)?, executor).await,
            Fixture::Delivery | Fixture::Courier => Err(Unrunnable::OtherMode(fixture)),
        }
    }
}

impl RunsFixtures for Asynchronous {
    async fn journey<E: Runs<Self>>(
        fixture: Fixture,
        model: &Model,
        executor: &mut E,
    ) -> Result<Journey, Unrunnable> {
        let data = Entries::of(model)?;
        match fixture {
            Fixture::Delivery => ran(delivery::instance(data)?, executor).await,
            Fixture::Courier => ran(courier::instance(data)?, executor).await,
            Fixture::Payment | Fixture::Warehouse => Err(Unrunnable::OtherMode(fixture)),
        }
    }
}

async fn ran<W: Send + Sync + 'static, M: Mode, E: Runs<M>>(
    instance: Instance<W, M>,
    executor: &mut E,
) -> Result<Journey, Unrunnable> {
    let journey_id = instance.journey_id().clone();
    let run = executor.run(instance).await.map(Ran::Journey);
    Ok(Journey {
        journey_id,
        reporters: BTreeMap::new(),
        run,
    })
}

/// The scenario's initial data, given to a fixture's instance builder as one value.
struct Entries {
    entries: Vec<(String, AnyValue)>,
}

impl Entries {
    fn of(model: &Model) -> Result<Self, ModelError> {
        let entries = model
            .initial_data
            .iter()
            .map(entry)
            .collect::<Result<_, _>>()?;
        Ok(Self { entries })
    }
}

fn entry((key, value): &(String, serde_json::Value)) -> Result<(String, AnyValue), ModelError> {
    Ok((key.clone(), Typed::of(value)?.erased()))
}

impl InitialData for Entries {
    fn entries(self) -> impl IntoIterator<Item = (String, AnyValue)> {
        self.entries
    }
}

#[cfg(test)]
mod tests {
    use itinera::executor::LocalExecutor;

    use super::*;

    #[test]
    fn naming_a_fixture_and_declaring_a_workflow_is_an_error_of_the_case() {
        let mut world = World {
            fixture: Some(Fixture::Payment),
            ..World::default()
        };
        world
            .model
            .declare("orders".to_owned(), vec!["charge".to_owned()])
            .unwrap();
        assert_eq!(named(&world), Err(ModelError::WorkflowDeclaredTwice));
    }

    #[test]
    fn a_fixture_runs_only_under_the_executor_of_its_mode() {
        let mut executor = LocalExecutor::new();
        let model = Model::default();
        let journey = Synchronous::journey(Fixture::Delivery, &model, &mut executor);
        assert!(matches!(
            futures::executor::block_on(journey),
            Err(Unrunnable::OtherMode(Fixture::Delivery))
        ));
    }
}
