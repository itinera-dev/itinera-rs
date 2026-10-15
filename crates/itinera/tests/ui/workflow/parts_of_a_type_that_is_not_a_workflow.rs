use itinera::error::Error;
use itinera::executor::LocalExecutor;
use itinera::journey::Read;
use itinera::policy::{OnSuccess, step_policy, workflow_policy};
use itinera::report::Reporter;
use itinera::step::{Outcome, Step, step};
use itinera::workflow::{Supplied, role, workflow};

trait Till {
    fn till(&self) -> &'static str;
}

#[derive(Step)]
struct Charge {
    #[input]
    amount: i64,
}

#[step]
impl Charge {
    fn run(self) -> Result<Outcome, Error> {
        if self.amount > 0 {
            Self::succeed()
        } else {
            Self::fail("nothing-to-charge")
        }
    }
}

#[derive(Default)]
struct Receipt;

#[step_policy]
impl Receipt {
    #[on_step_success]
    fn printed(&self, #[role] till: &dyn Till) -> Result<OnSuccess, Error> {
        till.till();
        Self::keep_default()
    }
}

#[derive(Default)]
struct Watch;

#[step_policy]
impl Watch {
    #[on_step_failure]
    fn failed(&self) -> Result<itinera::policy::OnFailure, Error> {
        Self::keep_default()
    }
}

#[derive(Default)]
struct Close;

#[workflow_policy]
impl Close {
    #[on_workflow_success]
    fn closed(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Default)]
struct Alarm;

#[workflow_policy]
impl Alarm {
    #[on_workflow_failure]
    fn raised(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Default)]
struct Silent;

impl Reporter for Silent {
    fn report(&mut self, _event: &itinera::event::Event) -> Result<(), Error> {
        Ok(())
    }
}

struct Shop {
    till: &'static str,
    fee: i64,
    region: Option<&'static str>,
}

#[role]
impl Till for Shop {
    fn till(&self) -> &'static str {
        self.till
    }
}

#[workflow]
impl Shop {
    #[input_adapter(steps = ["charge"])]
    fn pricing(
        &self,
        #[key] key: &str,
        data: &itinera::journey::DataBag,
    ) -> Result<Supplied, Error> {
        match data.read::<i64>(key) {
            Read::Present(amount) => Ok(Supplied::value(amount + self.fee)),
            _ => Ok(Supplied::FromDataBag),
        }
    }

    #[id_generator]
    fn basket_id(&self) -> Result<String, Error> {
        Ok(format!("{}-1", self.region.unwrap_or("nowhere")))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shop = Shop {
        till: "front",
        fee: 0,
        region: None,
    };
    let instance = itinera::workflow::Workflow::descriptor()
        .instance(shop)
        .initial_data(Basket { amount: 40 })
        .create()?;
    let result = LocalExecutor::new().run(instance)?;
    assert!(result.status.is_succeeded());
    Ok(())
}

#[derive(itinera::journey::InitialData)]
struct Basket {
    amount: i64,
}
