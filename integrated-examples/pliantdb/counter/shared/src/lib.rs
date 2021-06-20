use pliantdb::core::custom_api::CustomApi;
use serde::{Deserialize, Serialize};

/// The name of the database that the counter will use.
pub const DATABASE_NAME: &str = "counter";
/// The topic `PubSub` messages will be delivered on when the counter is
/// changed.
pub const COUNTER_CHANGED_TOPIC: &str = "counter-changed";

/// The API requests for this example.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "actionable-traits", derive(actionable::Actionable))]
pub enum Request {
    /// Increments the counter. No permissions needed.
    #[cfg_attr(feature = "actionable-traits", actionable(protection = "none"))]
    IncrementCounter,
}

/// The API responses for this example.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Response {
    /// The request to `IncrementCounter` resulted in the contained value.
    CounterIncremented(u64),
}

/// The [`CustomApi`] definition.
#[derive(Debug)]
pub enum ExampleApi {}

impl CustomApi for ExampleApi {
    type Request = Request;
    type Response = Response;
}
