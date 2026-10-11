//! What the equivalence tests compare: what a workflow lists, the result of one of its journeys,
//! and the events of that journey apart from their timestamps.

use std::sync::{Arc, Mutex};

use itinera::error::Error;
use itinera::event::Event;
#[cfg(feature = "async")]
use itinera::executor::AsyncLocalExecutor;
use itinera::executor::{LocalExecutor, Refusal};
use itinera::instance::{Instance, InstanceBuilder};
use itinera::journey::{DataBag, JourneyResult};
#[cfg(feature = "async")]
use itinera::mode::Asynchronous;
use itinera::mode::Mode;
use itinera::policy::Provides;
use itinera::report::Reporter;
use itinera::value::AnyValue;
use itinera::workflow::{WorkflowBuilder, WorkflowDescriptor};

/// The workflow the tests run, which holds nothing of its own.
pub(crate) struct Orders;

/// A role of the workflow: the desk that handles its orders. It is `Sync`, since an asynchronous
/// hook holds it across its awaits.
pub(crate) trait Desk: Sync {
    fn desk(&self) -> &'static str;
}

impl Desk for Orders {
    fn desk(&self) -> &'static str {
        "orders desk"
    }
}

impl Provides<dyn Desk> for Orders {
    fn role(&self) -> &(dyn Desk + 'static) {
        self
    }
}

/// The workflow's builder, with a journey ID that every journey shares, so that two journeys can
/// be compared.
pub(crate) fn orders() -> WorkflowBuilder<Orders> {
    WorkflowDescriptor::builder("orders").id_generator(first_order)
}

/// The asynchronous workflow's builder, with a journey ID that every journey shares.
#[cfg(feature = "async")]
pub(crate) fn async_orders() -> WorkflowBuilder<Orders, Asynchronous> {
    WorkflowDescriptor::async_builder("orders").id_generator(first_order)
}

fn first_order(_: &Orders, _: &DataBag) -> Result<String, Error> {
    Ok("order-1".to_owned())
}

/// What the tests compare of a workflow and one of its journeys.
#[derive(Debug, PartialEq)]
pub(crate) struct Journey {
    listing: String,
    result: String,
    output: Vec<String>,
    events: Vec<String>,
}

/// A reporter whose clones share the events they receive.
#[derive(Clone, Default)]
pub(crate) struct Recorder {
    events: Arc<Mutex<Vec<Event>>>,
}

impl Reporter for Recorder {
    fn report(&mut self, event: &Event) -> Result<(), Error> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

impl Recorder {
    /// Each event it received, written without its timestamp.
    fn trace(&self) -> Vec<String> {
        self.events.lock().unwrap().iter().map(untimed).collect()
    }
}

fn untimed(event: &Event) -> String {
    format!(
        "{} {} {} {:?}",
        event.sequence, event.journey_id, event.workflow, event.body
    )
}

/// Each key of the journey's output with its value, written as JSON: debugging a value writes only
/// its type.
fn output(result: &Result<JourneyResult, Refusal>) -> Vec<String> {
    let Ok(result) = result else {
        return Vec::new();
    };
    result
        .status
        .data()
        .into_iter()
        .flatten()
        .map(written)
        .collect()
}

pub(crate) fn written((key, value): (&String, &AnyValue)) -> String {
    format!("{key} = {}", serde_json::to_string(value).unwrap())
}

/// The initial data of a journey of the orders the tests run: each key is given when its field is
/// `Some`.
#[derive(Clone, Copy)]
pub(crate) struct Order {
    amount: Option<i64>,
    coupon: Option<&'static str>,
    gift_note: Option<Option<&'static str>>,
    outcome: Option<&'static str>,
    decision: Option<&'static str>,
}

const NOTHING: Order = Order {
    amount: None,
    coupon: None,
    gift_note: None,
    outcome: None,
    decision: None,
};

pub(crate) const A_GIFT: Order = Order {
    amount: Some(40),
    gift_note: Some(Some("happy birthday")),
    ..NOTHING
};

pub(crate) const NO_GIFT_WITH_A_COUPON: Order = Order {
    amount: Some(40),
    coupon: Some("spring"),
    gift_note: Some(None),
    ..NOTHING
};

pub(crate) const NOTHING_TO_CHARGE: Order = Order {
    amount: Some(0),
    ..A_GIFT
};

pub(crate) const NO_AMOUNT: Order = Order {
    amount: None,
    ..A_GIFT
};

pub(crate) const NOT_SAYING_WHETHER_IT_IS_A_GIFT: Order = Order {
    gift_note: None,
    ..A_GIFT
};

pub(crate) const AN_EMPTY_GIFT_NOTE: Order = Order {
    gift_note: Some(Some("")),
    ..A_GIFT
};

pub(crate) const SETTLED: Order = Order {
    outcome: Some("success"),
    ..NOTHING
};

pub(crate) const SETTLED_THEN_FINISHED: Order = Order {
    decision: Some("finish"),
    ..SETTLED
};

pub(crate) const SETTLED_THEN_REFUSED: Order = Order {
    decision: Some("fail"),
    ..SETTLED
};

pub(crate) const SETTLED_THEN_BROKEN: Order = Order {
    decision: Some("break"),
    ..SETTLED
};

pub(crate) const SETTLED_QUIETLY: Order = Order {
    outcome: Some("quiet"),
    ..NOTHING
};

pub(crate) const DECLINED: Order = Order {
    outcome: Some("failure"),
    ..NOTHING
};

pub(crate) const DECLINED_UNTIL_RETRIES_RAN_OUT: Order = Order {
    outcome: Some("retriable"),
    ..NOTHING
};

pub(crate) const CRASHED: Order = Order {
    outcome: Some("abnormal"),
    ..NOTHING
};

pub(crate) const CRASHED_THEN_REFUSED: Order = Order {
    decision: Some("fail"),
    ..CRASHED
};

impl Order {
    fn given_to<M: Mode>(self, builder: InstanceBuilder<Orders, M>) -> InstanceBuilder<Orders, M> {
        let builder = match self.amount {
            Some(amount) => builder.data("amount", amount),
            None => builder,
        };
        let builder = match self.coupon {
            Some(coupon) => builder.data("coupon", coupon.to_owned()),
            None => builder,
        };
        let builder = match self.gift_note {
            Some(note) => builder.data("gift_note", note.map(str::to_owned)),
            None => builder,
        };
        let builder = match self.outcome {
            Some(outcome) => builder.data("outcome", outcome.to_owned()),
            None => builder,
        };
        match self.decision {
            Some(decision) => builder.data("decision", decision.to_owned()),
            None => builder,
        }
    }
}

/// What a workflow lists, then the result and events of one journey of it with this initial
/// data, on the synchronous executor.
pub(crate) fn run(workflow: &WorkflowDescriptor<Orders>, order: Order) -> Journey {
    run_created(workflow, |recorder| instance(workflow, order, recorder))
}

/// What a workflow lists, then the result and events of one journey of it with this initial
/// data, on the asynchronous executor.
#[cfg(feature = "async")]
pub(crate) fn run_async(
    workflow: &WorkflowDescriptor<Orders, Asynchronous>,
    order: Order,
) -> Journey {
    run_created_async(workflow, |recorder| instance(workflow, order, recorder))
}

/// What a workflow lists, then the result and events of a journey of the instance made with the
/// recorder, on the synchronous executor.
pub(crate) fn run_created<W: Send + Sync + 'static>(
    workflow: &WorkflowDescriptor<W>,
    create: impl FnOnce(Recorder) -> Instance<W>,
) -> Journey {
    let recorder = Recorder::default();
    let result = LocalExecutor::new().run(create(recorder.clone()));
    Journey::of(workflow, &result, &recorder)
}

/// What a workflow lists, then the result and events of a journey of the instance made with the
/// recorder, on the asynchronous executor.
#[cfg(feature = "async")]
pub(crate) fn run_created_async<W: Send + Sync + 'static>(
    workflow: &WorkflowDescriptor<W, Asynchronous>,
    create: impl FnOnce(Recorder) -> Instance<W, Asynchronous>,
) -> Journey {
    let recorder = Recorder::default();
    let mut executor = AsyncLocalExecutor::new();
    let result = futures::executor::block_on(executor.run(create(recorder.clone())));
    Journey::of(workflow, &result, &recorder)
}

fn instance<M: Mode>(
    workflow: &WorkflowDescriptor<Orders, M>,
    order: Order,
    recorder: Recorder,
) -> Instance<Orders, M> {
    order
        .given_to(workflow.instance(Orders))
        .reporter(recorder)
        .create()
        .unwrap()
}

impl Journey {
    fn of<W: Send + Sync + 'static, M: Mode>(
        workflow: &WorkflowDescriptor<W, M>,
        result: &Result<JourneyResult, Refusal>,
        recorder: &Recorder,
    ) -> Self {
        Self {
            listing: format!("{:?}", workflow.listing()),
            result: format!("{result:?}"),
            output: output(result),
            events: recorder.trace(),
        }
    }
}
