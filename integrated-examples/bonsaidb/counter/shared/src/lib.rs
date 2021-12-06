use bonsaidb::core::custom_api::{CustomApi, Infallible};
use serde::{Deserialize, Serialize};

/// The name of the database that the counter will use.
pub const DATABASE_NAME: &str = "counter";

/// The API requests for this example.
#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "actionable-traits", derive(actionable::Actionable))]
pub enum Request {
    /// Request the current counter value.
    #[cfg_attr(feature = "actionable-traits", actionable(protection = "none"))]
    GetCounter,
    /// Increments the counter. No permissions needed.
    #[cfg_attr(feature = "actionable-traits", actionable(protection = "none"))]
    IncrementCounter,
}

/// The API responses for this example.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Response {
    /// The current value of the counter. Sent whenever requested or when the counter is updated.
    CounterValue(u64),
}

/// The [`CustomApi`] definition.
#[derive(Debug)]
pub enum ExampleApi {}

impl CustomApi for ExampleApi {
    type Error = Infallible;
    type Request = Request;
    type Response = Response;
}
