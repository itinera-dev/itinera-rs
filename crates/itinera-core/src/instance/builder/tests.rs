use super::*;
use crate::error::Error;
use crate::event::Event;
use crate::instance::WorkflowInstance;
use crate::instance::fixtures::{Audit, Made, Orders, entries, log, orders};
use crate::journey::InitialData;
use crate::workflow::WorkflowBuilder;

struct Announced;

impl Default for Announced {
    fn default() -> Self {
        log("announced made".to_owned());
        Self
    }
}

impl Reporter for Announced {
    fn report(&mut self, _event: &Event) -> Result<(), Error> {
        Ok(())
    }
}

fn logged(_: &Orders, _: &DataBag) -> Result<String, Error> {
    log("journey ID produced".to_owned());
    Ok("order-7".to_owned())
}

fn numbered(_: &Orders, data: &DataBag) -> Result<String, Error> {
    let number = data
        .get("number")
        .and_then(|value| value.downcast_ref::<i64>())
        .ok_or_else(|| Error::msg("no number"))?;
    Ok(format!("order-{number}"))
}

fn numbered_orders() -> WorkflowBuilder<Orders> {
    orders().id_generator(numbered)
}

fn is_uuid_v4(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(is_version_4)
}

fn is_version_4(uuid: uuid::Uuid) -> bool {
    uuid.get_version_num() == 4
}

#[test]
fn without_a_generator_the_journey_id_is_a_uuid_v4() {
    let orders = orders().build().unwrap();
    let first = orders.instance(Orders::new()).create().unwrap();
    let second = orders.instance(Orders::new()).create().unwrap();

    assert!(is_uuid_v4(first.journey_id().as_ref()));
    assert_ne!(first.journey_id(), second.journey_id());
}

#[test]
fn the_generator_produces_the_journey_id_from_the_initial_data() {
    let orders = numbered_orders().build().unwrap();
    let instance = orders
        .instance(Orders::new())
        .data("number", 7_i64)
        .create()
        .unwrap();

    assert_eq!(instance.journey_id().to_string(), "order-7");
}

#[test]
fn a_generator_that_fails_makes_creating_the_instance_fail() {
    let orders = numbered_orders().build().unwrap();
    let error = orders.instance(Orders::new()).create().unwrap_err();

    assert!(matches!(error, InstanceError::JourneyId(_)));
    assert_eq!(
        error.to_string(),
        "the journey ID could not be produced: no number"
    );
}

#[test]
fn the_instance_holds_every_reporter_those_the_workflow_lists_first() {
    let orders = orders().reporter::<Audit>().build().unwrap();
    let mut instance = orders
        .instance(Orders::new())
        .reporter(Made {
            name: "request log",
        })
        .reporter(Made { name: "metrics" })
        .create()
        .unwrap();

    let reporters = instance.take_reporters();
    assert!(instance.take_reporters().is_empty());
    for mut reporter in reporters {
        reporter
            .set_global_metadata("tenant", &AnyValue::new("acme".to_string()))
            .unwrap();
    }
    assert_eq!(
        entries(),
        [
            "audit got tenant",
            "request log got tenant",
            "metrics got tenant"
        ]
    );
}

#[test]
fn a_listed_reporter_is_made_once_the_journey_id_is_produced() {
    let orders = orders()
        .reporter::<Announced>()
        .id_generator(logged)
        .build()
        .unwrap();
    orders.instance(Orders::new()).create().unwrap();

    assert_eq!(entries(), ["journey ID produced", "announced made"]);
}

struct NewOrder {
    price: i64,
    note: &'static str,
}

impl InitialData for NewOrder {
    fn entries(self) -> impl IntoIterator<Item = (String, AnyValue)> {
        [
            ("price".to_owned(), AnyValue::new(self.price)),
            ("note".to_owned(), AnyValue::new(self.note.to_owned())),
        ]
    }
}

fn price(instance: &Instance<Orders>) -> Option<i64> {
    instance.data_bag().get("price")?.downcast_ref().copied()
}

#[test]
fn initial_data_gives_each_of_its_keys_replacing_what_was_given_before() {
    let orders = orders().build().unwrap();
    let instance = orders
        .instance(Orders::new())
        .data("price", 7_i64)
        .data("customer", "ana".to_owned())
        .initial_data(NewOrder {
            price: 42,
            note: "gift",
        })
        .create()
        .unwrap();

    assert_eq!(
        instance.data_bag().keys().collect::<Vec<_>>(),
        ["customer", "note", "price"]
    );
    assert_eq!(price(&instance), Some(42));
}

#[cfg(feature = "async")]
mod asynchronous {
    use super::*;
    use crate::instance::fixtures::note;
    use crate::mode::Asynchronous;
    use crate::report::{AsyncReporter, BoxedReporter};

    #[derive(Default)]
    struct Forward;

    impl AsyncReporter for Forward {
        fn set_global_metadata(&mut self, key: &str, _value: &AnyValue) -> Result<(), Error> {
            note("forward", key);
            Ok(())
        }

        async fn report(&mut self, _event: &Event) -> Result<(), Error> {
            Ok(())
        }
    }

    fn kind(reporter: &BoxedReporter) -> String {
        format!("{reporter:?}")
    }

    #[test]
    fn an_asynchronous_instance_holds_reporters_of_both_kinds_in_order() {
        let orders: WorkflowDescriptor<Orders, Asynchronous> =
            WorkflowDescriptor::async_builder("orders")
                .reporter::<Audit>()
                .async_reporter::<Forward>()
                .build()
                .unwrap();
        let mut instance = orders
            .instance(Orders::new())
            .reporter(Made {
                name: "request log",
            })
            .async_reporter(Forward)
            .create()
            .unwrap();

        let reporters = instance.take_reporters();
        let kinds: Vec<String> = reporters.iter().map(kind).collect();
        assert_eq!(
            kinds,
            [
                r#"BoxedReporter("sync")"#,
                r#"BoxedReporter("async")"#,
                r#"BoxedReporter("sync")"#,
                r#"BoxedReporter("async")"#,
            ]
        );
        for mut reporter in reporters {
            reporter
                .set_global_metadata("tenant", &AnyValue::new("acme".to_string()))
                .unwrap();
        }
        assert_eq!(
            entries(),
            [
                "audit got tenant",
                "forward got tenant",
                "request log got tenant",
                "forward got tenant",
            ]
        );
    }
}
