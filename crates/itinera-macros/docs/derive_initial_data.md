Gives the data a journey starts with as a struct: each field is one key of the data bag, given to an instance with `.initial_data(...)`.

A field's key is its name, unless `#[key("...")]` names it. Its type is a value. A field of type `Option<T>` that is `None` leaves its key out of the data bag, unless it is marked `#[required]`: the key then holds the `Option` itself, `None` included. `#[required]` on a field of any other type does not compile, since its key is always given. The macro reads only the tokens written, so an alias of `Option` is given as it is.

The struct has named fields, or none, and is not generic. The derive implements `InitialData`, which an application may also implement by hand.

# Examples

```
use itinera::instance::WorkflowInstance;
use itinera::journey::InitialData;
use itinera::workflow::WorkflowDescriptor;

#[derive(InitialData)]
struct NewOrder {
    price: i64,
    #[key("discount_code")]
    code: Option<String>,
    #[required]
    gift_note: Option<String>,
}

struct Orders;

let orders = WorkflowDescriptor::builder("orders").build()?;
let instance = orders
    .instance(Orders)
    .initial_data(NewOrder {
        price: 42,
        code: None,
        gift_note: None,
    })
    .create()?;
assert_eq!(instance.data_bag().keys().collect::<Vec<_>>(), ["gift_note", "price"]);
# Ok::<(), Box<dyn std::error::Error>>(())
```
