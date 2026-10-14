Lets a trait with `async fn` methods be used as a trait object, as asynchronous workflows use their services and roles.

A trait with a plain `async fn` cannot be a trait object, so a step could not take it as `&dyn Payments`, nor a hook as a role. With `#[async_trait]` on the trait and on each of its implementations, it can, and every call of such a method is awaited. The attribute hands its item and arguments to the attribute of the [`async-trait`](https://docs.rs/async-trait) crate, so applications need not depend on that crate themselves. The methods' futures are `Send`, as the futures of asynchronous steps and hooks must be. It is available with the `async` feature.

# Examples

```
use itinera::mode::async_trait;

#[async_trait]
trait Payments: Send + Sync {
    async fn charge(&self, cents: u64) -> Result<u64, String>;
}

struct Gateway;

#[async_trait]
impl Payments for Gateway {
    async fn charge(&self, cents: u64) -> Result<u64, String> {
        Ok(cents)
    }
}

async fn checkout(payments: &dyn Payments) -> Result<u64, String> {
    payments.charge(1250).await
}

assert_eq!(futures::executor::block_on(checkout(&Gateway)), Ok(1250));
```
