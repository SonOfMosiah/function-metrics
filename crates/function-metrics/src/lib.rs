//! Prometheus-native function instrumentation built on the [`metrics`] facade.
//!
//! The [`macro@function_metrics`] attribute currently records a duration histogram
//! in fractional seconds. Its interface is designed to grow into a coherent
//! family of call, error, and in-flight metrics without changing how callers
//! name operations or attach application labels.
//!
//! # Example
//!
//! ```
//! use function_metrics::function_metrics;
//!
//! #[function_metrics(name = "parse_config", labels(format))]
//! fn parse_config(format: &str) -> usize {
//!     format.len()
//! }
//!
//! assert_eq!(parse_config("toml"), 4);
//! ```

extern crate self as function_metrics;

pub use function_metrics_macros::function_metrics;

#[doc(hidden)]
pub mod __private {
    use std::time::Duration;

    pub use metrics::Label;

    pub fn record_duration(name: &'static str, labels: Vec<Label>, duration: Duration) {
        metrics::histogram!(name, labels).record(duration);
    }
}
