//! Prometheus-native function instrumentation built on the [`metrics`] facade.
//!
//! The [`macro@function_metrics`] attribute currently records a duration histogram
//! in fractional seconds. Its interface is designed to grow into a coherent
//! family of call, error, and in-flight metrics without changing how callers
//! name operations or attach application labels.
//! Durations are recorded on normal returns, panics, and cancellation after a
//! future has started polling.
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
    use std::{
        future::Future,
        panic::{AssertUnwindSafe, catch_unwind},
        time::{Duration, Instant},
    };

    pub use metrics::Label;

    struct DurationGuard {
        name: &'static str,
        labels: Option<Vec<Label>>,
        started: Instant,
    }

    impl DurationGuard {
        fn complete(mut self) {
            self.record();
        }

        fn record(&mut self) {
            if let Some(labels) = self.labels.take() {
                record_duration(self.name, labels, self.started.elapsed());
            }
        }
    }

    impl Drop for DurationGuard {
        fn drop(&mut self) {
            if std::thread::panicking() {
                let _ = catch_unwind(AssertUnwindSafe(|| self.record()));
            } else {
                self.record();
            }
        }
    }

    fn start_duration(name: &'static str, labels: Vec<Label>) -> DurationGuard {
        DurationGuard {
            name,
            labels: Some(labels),
            started: Instant::now(),
        }
    }

    fn record_duration(name: &'static str, labels: Vec<Label>, duration: Duration) {
        metrics::histogram!(name, labels).record(duration);
    }

    pub fn measure_sync<R>(name: &'static str, labels: Vec<Label>, body: impl FnOnce() -> R) -> R {
        let guard = start_duration(name, labels);
        let result = body();
        guard.complete();
        result
    }

    pub async fn measure_future<F>(name: &'static str, labels: Vec<Label>, future: F) -> F::Output
    where
        F: Future,
    {
        let guard = start_duration(name, labels);
        let result = future.await;
        guard.complete();
        result
    }
}
