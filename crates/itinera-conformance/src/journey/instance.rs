//! A new workflow instance of the scenario's workflow, with its reporters, its initial data and
//! global metadata, and what the scenario asks of test support: its recording reporter and the
//! fakes in place of providers.

use std::collections::BTreeMap;

use itinera::instance::{Instance, InstanceBuilder};
use itinera::mode::Mode;
use itinera::report::RecordingReporter;

use super::Unrunnable;
use super::initial::{with_data, with_global_metadata};
use crate::declaration::{Admission, Declares, ScriptedWorkflow, declared};
use crate::record::Recorder;
use crate::role::ProvidedRoles;
use crate::service::with_fake;
use crate::testing::unless_stopped;
use crate::world::World;

/// A new instance, with the recorders of the reporters the workflow lists, by name.
pub(super) struct Created<M: Mode> {
    pub(super) instance: Instance<ScriptedWorkflow, M>,
    pub(super) reporters: BTreeMap<String, Recorder>,
}

/// The reporters an instance has before the recording reporter of test support.
#[derive(PartialEq, Eq)]
pub(super) enum Reporters {
    /// Those the workflow lists.
    Listed,
    /// The runner's own recorder, then those the workflow lists, for a run whose executor has
    /// no dispatcher factory from the runner.
    OwnFirst,
}

/// Declares and builds the workflow, then creates a new instance of it, unless the workflow is
/// refused, or test support stops the test while the instance is made.
pub(super) async fn created<M: Declares>(
    world: &mut World,
    reporters: Reporters,
) -> Result<Option<Created<M>>, Unrunnable> {
    let descriptor = match declared::<M>(&world.model, &world.steps_run, &world.witness)?.build() {
        Ok(descriptor) => descriptor,
        Err(violations) => {
            world.admission = Some(Admission::Refused(violations));
            return Ok(None);
        }
    };
    world.admission = Some(Admission::Admitted(descriptor.listing()));
    let model = &world.model;
    let workflow = model.workflow()?;
    let names = &workflow.reporters;
    let recorders: Vec<Recorder> = names
        .iter()
        .map(|name| world.recorders.renewed(name, model))
        .collect();
    let own = (reporters == Reporters::OwnFirst).then(|| world.recorders.own());
    let recording = model
        .testing
        .recording_reporter
        .then(RecordingReporter::default);
    world.tested.recording_reporter.clone_from(&recording);
    let fake_calls = world.tested.counting(&model.testing.fakes);
    let roles = ProvidedRoles::new(workflow.roles.clone(), world.witness.clone());
    let made = unless_stopped(async {
        let instance = descriptor.instance(ScriptedWorkflow::new(roles));
        let instance = own
            .into_iter()
            .chain(recorders.iter().cloned())
            .fold(instance, InstanceBuilder::reporter);
        let instance = recording
            .into_iter()
            .fold(instance, InstanceBuilder::reporter);
        let instance = model
            .testing
            .fakes
            .iter()
            .zip(fake_calls)
            .fold(instance, |instance, (fake, calls)| {
                with_fake(instance, fake, calls)
            });
        let instance = model.initial_data.iter().try_fold(instance, with_data)?;
        let instance = model
            .initial_global_metadata
            .iter()
            .try_fold(instance, with_global_metadata)?;
        Ok::<_, Unrunnable>(instance.create()?)
    })
    .await;
    match made {
        Ok(instance) => Ok(Some(Created {
            instance: instance?,
            reporters: names.iter().cloned().zip(recorders).collect(),
        })),
        Err(stopped) => {
            world.tested.stopped = Some(stopped);
            Ok(None)
        }
    }
}
