use serde_json::json;

use super::*;

#[test]
fn a_keys_values_are_every_value_the_scenario_gives_it() {
    let mut model = Model::default();
    model
        .declare("orders".to_owned(), vec!["charge".to_owned()])
        .unwrap();
    model.initial_data.push(("card".to_owned(), json!("4111")));
    let charge = model.step_mut("charge").unwrap();
    charge.actions.push(StepAction::ContributeThenChange {
        key: "token".to_owned(),
        value: json!("first"),
        changed: json!("second"),
    });
    charge.attempts = Some(vec![Attempt {
        outcome: step::AttemptOutcome::Success,
        contributes: vec![("token".to_owned(), json!("third"))],
    }]);
    model
        .define("record".to_owned(), "on step success", true)
        .unwrap();
    model
        .hook_mut("record", "on step success")
        .unwrap()
        .actions
        .push(HookAction::Contribute {
            key: "token".to_owned(),
            value: json!("fourth"),
        });
    assert_eq!(model.values_of("card"), [&json!("4111")]);
    assert_eq!(
        model.values_of("token"),
        [
            &json!("first"),
            &json!("second"),
            &json!("third"),
            &json!("fourth")
        ]
    );
    assert!(model.values_of("note").is_empty());
}

#[test]
fn a_step_is_scripted_once_whatever_times_its_name_is_declared() {
    let mut model = Model::default();
    model
        .declare(
            "orders".to_owned(),
            vec!["charge".to_owned(), "charge".to_owned()],
        )
        .unwrap();
    model.step_mut("charge").unwrap().retries = 2;
    let workflow = model.workflow().unwrap();
    assert_eq!(workflow.steps, ["charge", "charge"]);
    assert_eq!(workflow.scripts.len(), 1);
    assert!(model.step_mut("ship").is_err());
}

#[test]
fn a_policys_kind_is_set_by_the_sentence_that_defines_its_first_hook() {
    let mut model = Model::default();
    model
        .define("audit".to_owned(), "on step success", true)
        .unwrap();
    model
        .define("audit".to_owned(), "on step retry", true)
        .unwrap();
    assert!(
        model
            .define("audit".to_owned(), "on workflow success", false)
            .is_err()
    );
    assert!(
        model
            .define("notify".to_owned(), "on step success", false)
            .is_err()
    );
    assert!(model.hook_mut("audit", "on step retry").is_ok());
    assert_eq!(
        model.hook_mut("audit", "on step failure").unwrap_err(),
        ModelError::HookNotDefined("audit".to_owned(), Hook::named("on step failure").unwrap())
    );
}

fn ordering() -> Model {
    let mut model = Model::default();
    model
        .declare("orders".to_owned(), vec!["charge".to_owned()])
        .unwrap();
    model
}

#[test]
fn an_input_adapter_defined_before_the_workflow_is_declared_by_its_binding() {
    let mut model = Model::default();
    model.adapter_mut("pricing").unwrap().data_bag = true;
    model
        .declare("orders".to_owned(), vec!["charge".to_owned()])
        .unwrap();

    model
        .bind_adapter("pricing", vec!["charge".to_owned()])
        .unwrap();

    let adapters = &model.workflow().unwrap().adapters;
    assert_eq!(adapters.len(), 1);
    assert!(adapters[0].data_bag);
    assert_eq!(adapters[0].steps, ["charge"]);
    assert_eq!(model.require_bound(), Ok(()));
}

fn metering() -> Provider {
    Provider::new(
        "usage".to_owned(),
        "meter".to_owned(),
        ServiceType::Meter,
        Scope::Attempt,
    )
}

fn bind_usage(model: &mut Model) -> Result<(), ModelError> {
    model.bind_provider("usage", vec!["charge".to_owned()])
}

#[test]
fn a_service_provider_is_bound_only_once_it_is_defined() {
    let mut model = ordering();
    model.define_provider(metering());

    assert_eq!(bind_usage(&mut model), Ok(()));
    assert_eq!(
        bind_usage(&mut model),
        Err(ModelError::NotDefined("usage".to_owned()))
    );
    assert_eq!(model.workflow().unwrap().providers.len(), 1);
}

#[test]
fn a_building_block_defined_and_never_bound_is_a_mistake_of_the_case() {
    let mut model = ordering();
    model.define_provider(metering());
    assert_eq!(
        model.require_bound(),
        Err(ModelError::NotBound("service provider", "usage".to_owned()))
    );

    model.adapter_mut("pricing").unwrap();
    assert_eq!(
        model.require_bound(),
        Err(ModelError::NotBound("input adapter", "pricing".to_owned()))
    );
}
