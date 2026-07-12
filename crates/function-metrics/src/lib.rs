//! Prometheus-native function instrumentation built on the [`metrics`] facade.
//!
//! [`macro@function_metrics`] records duration histograms and can optionally
//! record call and returned-error counters. Metric families use conventional
//! `_duration_seconds`, `_calls_total`, and `_errors_total` suffixes.
//!
//! # Example
//!
//! ```
//! use function_metrics::function_metrics;
//!
//! #[function_metrics(
//!     name = "parse_config",
//!     metrics(duration, calls, errors),
//!     labels(format),
//! )]
//! fn parse_config(format: &str) -> Result<usize, &'static str> {
//!     Ok(format.len())
//! }
//!
//! assert_eq!(parse_config("toml"), Ok(4));
//! ```
//!
//! Omitting `metrics(...)` preserves the duration-only behavior from 0.1.
//! Applications own recorder installation and histogram bucket policy.

extern crate self as function_metrics;

pub use function_metrics_macros::function_metrics;

#[doc(hidden)]
pub mod __private {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        time::Instant,
    };

    pub use metrics::Label;

    pub struct MetricDescriptor {
        name: &'static str,
        description: &'static str,
    }

    impl MetricDescriptor {
        pub const fn new(name: &'static str, description: &'static str) -> Self {
            Self { name, description }
        }
    }

    pub struct InvocationGuard {
        duration: Option<metrics::Histogram>,
        calls: Option<metrics::Counter>,
        errors: Option<metrics::Counter>,
        started: Instant,
        recorded: bool,
    }

    impl InvocationGuard {
        pub fn start(
            duration: Option<MetricDescriptor>,
            calls: Option<MetricDescriptor>,
            errors: Option<MetricDescriptor>,
            labels: Vec<Label>,
        ) -> Self {
            if let Some(descriptor) = &duration {
                metrics::describe_histogram!(descriptor.name, metrics::Unit::Seconds, descriptor.description);
            }
            if let Some(descriptor) = &calls {
                metrics::describe_counter!(descriptor.name, descriptor.description);
            }
            if let Some(descriptor) = &errors {
                metrics::describe_counter!(descriptor.name, descriptor.description);
            }

            let duration = duration.map(|descriptor| metrics::histogram!(descriptor.name, labels.clone()));
            let calls = calls.map(|descriptor| metrics::counter!(descriptor.name, labels.clone()));
            let errors = errors.map(|descriptor| metrics::counter!(descriptor.name, labels));

            Self {
                duration,
                calls,
                errors,
                started: Instant::now(),
                recorded: false,
            }
        }

        pub fn complete(mut self, is_error: bool) {
            if is_error {
                if let Some(errors) = &self.errors {
                    errors.increment(1);
                }
            }
            self.record();
        }

        fn record(&mut self) {
            if self.recorded {
                return;
            }
            self.recorded = true;
            if let Some(duration) = &self.duration {
                duration.record(self.started.elapsed());
            }
            if let Some(calls) = &self.calls {
                calls.increment(1);
            }
        }
    }

    impl Drop for InvocationGuard {
        fn drop(&mut self) {
            if std::thread::panicking() {
                let _ = catch_unwind(AssertUnwindSafe(|| self.record()));
            } else {
                self.record();
            }
        }
    }
}
