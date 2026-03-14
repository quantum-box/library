//! Provider-specific event processors for webhook handling.

pub mod github;
pub mod hubspot;
pub mod linear;
pub mod notion;
pub mod square;
pub mod stripe;

pub use github::GitHubEventProcessor;
pub use hubspot::HubSpotEventProcessor;
pub use linear::LinearEventProcessor;
pub use notion::NotionEventProcessor;
pub use square::SquareEventProcessor;
pub use stripe::StripeEventProcessor;
