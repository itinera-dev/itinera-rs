//! The workflow of the `orders` example, declared with the builder alone, as an application
//! without the `macros` feature declares it. Run it with
//! `cargo run -p itinera --example orders_by_hand --no-default-features`.

use itinera::error::Result;
use itinera::event::Event;
use itinera::executor::LocalExecutor;
use itinera::journey::{Contributor, DataBag};
use itinera::policy::{
    HookNeeds, InputAdapter, OnStepSuccess, OnSuccess, OnWorkflowSuccess, Provider, Provides,
    Requested, StepPolicyDescriptor, StepSuccess, WorkflowPolicyDescriptor, WorkflowSuccess,
};
use itinera::report::Reporter;
use itinera::service::{PerAttempt, Provided, Service, ServiceProviderDescriptor, provide};
use itinera::step::{
    Attempt, Input, OptionalInput, Outcome, Resolved, Step, StepDescriptor, StepFactory, StepNeeds,
    step_name,
};
use itinera::workflow::{InputAdapterDescriptor, Supplied, WorkflowDescriptor};

const AMOUNT: Input<i64> = Input::new("amount");
const CHARGED: Input<i64> = Input::new("charged");
const PRICE: Input<i64> = Input::new("price");
const DISCOUNT: OptionalInput<i64> = OptionalInput::new("discount");
const PAYMENT: Input<String> = Input::new("payment");
const PAYMENTS: Service<dyn PaymentGateway> = Service::new("payments");

/// Takes payments: the service a charge is paid through.
trait PaymentGateway: Send + Sync {
    /// Takes the amount, and says how it was paid.
    fn take(&self, amount: i64) -> String;
}

/// The shop's card terminal, lent to one attempt to charge.
struct Terminal {
    shop: &'static str,
    attempt: Attempt,
}

impl PaymentGateway for Terminal {
    fn take(&self, amount: i64) -> String {
        format!(
            "{amount} paid at the {} terminal, attempt {}",
            self.shop, self.attempt
        )
    }
}

/// Charges the amount the workflow's pricing works out, through the payment gateway. A charge of
/// nothing is declined, which may be retried.
struct Charge<'a> {
    amount: i64,
    payments: &'a dyn PaymentGateway,
    contributor: &'a mut Contributor,
}

impl Step for Charge<'_> {
    fn run(self) -> Result<Outcome> {
        if self.amount <= 0 {
            return Ok(Outcome::failure("declined"));
        }
        self.contributor
            .contribute("payment", self.payments.take(self.amount));
        self.contributor.contribute("charged", self.amount);
        Ok(Outcome::success())
    }
}

/// Builds a charge for each attempt.
struct Charging;

impl StepFactory for Charging {
    type Step<'a> = Charge<'a>;

    fn needs(&self) -> StepNeeds {
        StepNeeds::new()
            .input(&AMOUNT)
            .contributor()
            .service(&PAYMENTS)
    }

    fn build<'a>(&'a self, got: &mut Resolved<'a>) -> Result<Charge<'a>> {
        Ok(Charge {
            amount: got.input(&AMOUNT)?,
            payments: got.service(&PAYMENTS)?,
            contributor: got.contributor()?,
        })
    }
}

/// Ships the order: a step that needs nothing, whose function is its own factory.
fn ship() -> Result<Outcome> {
    Ok(Outcome::success())
}

/// Notes each amount charged.
struct Audit;

impl OnStepSuccess<Orders> for Audit {
    fn needs() -> HookNeeds<StepSuccess> {
        HookNeeds::new().from_step(&CHARGED).contributor()
    }

    fn on_step_success(&self, mut got: Requested<'_, Orders, StepSuccess>) -> Result<OnSuccess> {
        let charged = got.from_step(&CHARGED)?;
        got.contributor()?
            .contribute("audit", format!("charged {charged}"));
        Ok(OnSuccess::KeepDefault)
    }
}

/// A role the shop provides: telling a customer something.
trait Messages {
    fn tell(&self, message: &str);
}

/// Tells the customer once the order is paid and shipped.
struct Notify;

impl OnWorkflowSuccess<Orders> for Notify {
    fn needs() -> HookNeeds<WorkflowSuccess> {
        HookNeeds::new()
            .from_workflow(&CHARGED)
            .from_workflow(&PAYMENT)
    }

    fn on_workflow_success(&self, mut got: Requested<'_, Orders, WorkflowSuccess>) -> Result<()> {
        let charged = got.from_workflow(&CHARGED)?;
        let payment = got.from_workflow(&PAYMENT)?;
        let messages: &dyn Messages = got.role();
        messages.tell(&format!(
            "order {}: paid {charged}, {payment}",
            got.journey_id()
        ));
        Ok(())
    }
}

/// Prints each event of every journey.
#[derive(Default)]
struct Printer;

impl Reporter for Printer {
    fn report(&mut self, event: &Event) -> Result<()> {
        println!("{} {}", event.journey_id, event.kind());
        Ok(())
    }
}

struct Orders {
    shop: &'static str,
}

impl Messages for Orders {
    fn tell(&self, message: &str) {
        println!("{}: {message}", self.shop);
    }
}

impl Provides<dyn Messages> for Orders {
    fn role(&self) -> &(dyn Messages + 'static) {
        self
    }
}

/// Works out the amount to charge: the price, less any discount.
fn pricing(_: &Orders, mut got: Requested<'_, Orders, InputAdapter>) -> Result<Supplied> {
    let price = got.from_workflow(&PRICE)?;
    let discount = got.optional_from_workflow(&DISCOUNT)?;
    Ok(Supplied::value(price - discount.unwrap_or_default()))
}

/// Lends each attempt to charge the shop's card terminal.
fn payments(
    orders: &Orders,
    got: Requested<'_, Orders, Provider<PerAttempt>>,
) -> Result<Provided<dyn PaymentGateway>> {
    Ok(provide!(Terminal {
        shop: orders.shop,
        attempt: got.attempt(),
    }))
}

/// Names each journey after the shop and the order.
fn order_id(orders: &Orders, data: &DataBag) -> Result<String> {
    let order: String = data.required("order")?;
    Ok(format!("{}-{order}", orders.shop))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let audit = StepPolicyDescriptor::new("audit", || Audit).on_step_success();
    let notify = WorkflowPolicyDescriptor::new("notify", || Notify).on_workflow_success();
    let adapter = InputAdapterDescriptor::new("pricing", step_name!("charge"), pricing).needing(
        HookNeeds::new()
            .from_workflow(&PRICE)
            .optional_from_workflow(&DISCOUNT),
    );
    let payments = ServiceProviderDescriptor::per_attempt("payments", &PAYMENTS, payments)
        .step(step_name!("charge"));
    let orders = WorkflowDescriptor::builder("orders")
        .step(
            StepDescriptor::new(step_name!("charge"), Charging)
                .retry_budget(1)
                .policy(audit),
        )
        .step(StepDescriptor::new(step_name!("ship"), ship))
        .policy(notify)
        .reporter::<Printer>()
        .input_adapter(adapter)
        .provider(payments)
        .id_generator(order_id)
        .build()?;
    let mut executor = LocalExecutor::new();
    for (order, price, discount) in [("1", 40_i64, 5_i64), ("2", 10, 10)] {
        let instance = orders
            .instance(Orders { shop: "corner" })
            .data("order", order.to_owned())
            .data("price", price)
            .data("discount", discount)
            .create()?;
        let result = executor.run(instance)?;
        println!("{}: {}", result.journey_id, result.status.kind());
    }
    Ok(())
}
