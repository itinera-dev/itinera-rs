//! The roles the scenario's workflow provides to its policies' hooks.

use std::collections::BTreeMap;

use itinera::error::Error;

use crate::model::Role;
use crate::witness::{Operation, Witness};

/// The scenario's roles, which a hook calls by name: each operation records its call, then
/// fails if the scenario says it throws.
pub(crate) trait Roles: Send + Sync {
    fn call(&self, role: &str, operation: &str) -> Result<(), Error>;
}

/// The roles the workflow provides, as the scenario declares them.
#[derive(Debug, Default)]
pub(crate) struct ProvidedRoles {
    roles: BTreeMap<String, Role>,
    witness: Witness,
}

impl ProvidedRoles {
    pub(crate) fn new(roles: BTreeMap<String, Role>, witness: Witness) -> Self {
        Self { roles, witness }
    }
}

impl Roles for ProvidedRoles {
    fn call(&self, role: &str, operation: &str) -> Result<(), Error> {
        let provided = self
            .roles
            .get(role)
            .filter(|provided| provided.operations.contains(operation))
            .ok_or_else(|| no_operation(role, operation))?;
        self.witness.operated(Operation {
            role: role.to_owned(),
            operation: operation.to_owned(),
        });
        if provided.failing.contains(operation) {
            Err(Error::msg(format!(
                "the operation {operation} of the role {role} throws"
            )))
        } else {
            Ok(())
        }
    }
}

/// The error of a call to an operation the workflow does not provide.
fn no_operation(role: &str, operation: &str) -> Error {
    Error::msg(format!("the role {role} has no operation {operation}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn notifier(failing: &[&str], witness: &Witness) -> ProvidedRoles {
        let role = Role {
            operations: BTreeSet::from(["notify".to_owned()]),
            failing: failing.iter().copied().map(str::to_owned).collect(),
        };
        ProvidedRoles::new(
            BTreeMap::from([("notifier".to_owned(), role)]),
            witness.clone(),
        )
    }

    fn notify() -> Operation {
        Operation {
            role: "notifier".to_owned(),
            operation: "notify".to_owned(),
        }
    }

    #[test]
    fn a_call_is_recorded_before_its_operation_throws() {
        let witness = Witness::default();
        let error = notifier(&["notify"], &witness)
            .call("notifier", "notify")
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "the operation notify of the role notifier throws"
        );
        assert_eq!(witness.operations(), [notify()]);
    }

    #[test]
    fn a_call_of_an_operation_the_workflow_does_not_provide_fails_unrecorded() {
        let witness = Witness::default();
        let roles = notifier(&[], &witness);
        assert!(roles.call("notifier", "page").is_err());
        assert!(roles.call("pager", "notify").is_err());
        assert!(witness.operations().is_empty());
    }
}
