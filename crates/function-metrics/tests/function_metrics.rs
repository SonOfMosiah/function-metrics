use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use async_trait::async_trait;
use function_metrics::function_metrics;
use metrics::{Counter, Gauge, Histogram, Key, KeyName, Label, Metadata, Recorder, SharedString, Unit};
use metrics_util::debugging::{DebugValue, DebuggingRecorder};

#[function_metrics(name = "test_sync", labels(shard_id, provider = "test"))]
fn timed_sync(shard_id: u64) -> u64 {
    std::thread::sleep(Duration::from_millis(2));
    shard_id
}

#[function_metrics(
    name = "instrumented_result",
    metrics(duration, calls, errors),
    labels(provider = "test")
)]
fn instrumented_result(succeed: bool) -> Result<(), &'static str> {
    if succeed { Ok(()) } else { Err("failed") }
}

type ClassifiedResult = u16;

fn status_is_error(status: &ClassifiedResult) -> bool {
    *status >= 400
}

#[function_metrics(
    name = "classified_result",
    metrics(calls, errors),
    error_classifier = status_is_error
)]
fn classified_result(status: u16) -> ClassifiedResult {
    status
}

#[function_metrics(name = "fallible_async", metrics(calls, errors))]
async fn fallible_async() -> Result<(), &'static str> {
    Err::<(), _>("question mark")?;
    Ok(())
}

#[function_metrics(name = "panicking_result", metrics(calls, errors))]
fn panicking_result() -> Result<(), &'static str> {
    panic!("panic is not a returned error")
}

#[function_metrics(name = "pending_calls", metrics(calls, errors))]
async fn pending_calls() -> Result<(), &'static str> {
    std::future::pending::<()>().await;
    Ok(())
}

#[function_metrics(name = "described_operation", metrics(duration, calls, errors))]
fn described_operation() -> Result<(), &'static str> {
    Ok(())
}

#[function_metrics(labels(shard_id))]
async fn timed_async(shard_id: String) -> String {
    tokio::time::sleep(Duration::from_millis(2)).await;
    shard_id
}

#[function_metrics(name = "early_sync")]
fn early_sync(should_return: bool) -> Result<(), &'static str> {
    if should_return {
        return Err("early return");
    }
    Ok(())
}

#[function_metrics(name = "question_mark_async")]
async fn question_mark_async() -> Result<(), &'static str> {
    Err::<(), _>("question mark")?;
    Ok(())
}

#[function_metrics(name = "panicking_sync")]
fn panicking_sync() {
    if std::hint::black_box(true) {
        panic!("timed panic");
    }
}

#[function_metrics(name = "cancelled_async")]
async fn pending_async() {
    std::future::pending::<()>().await;
}

struct TimedService;

#[async_trait]
trait TimedOperation {
    #[function_metrics(name = "test_async_trait", labels(shard_id))]
    async fn execute(&self, shard_id: u64) -> u64 {
        tokio::time::sleep(Duration::from_millis(2)).await;
        shard_id
    }

    #[function_metrics(name = "cancelled_async_trait", labels(shard_id))]
    async fn pending(&self, shard_id: u64) {
        let _ = shard_id;
        std::future::pending::<()>().await;
    }

    #[function_metrics(name = "fallible_async_trait", metrics(duration, calls, errors), labels(shard_id))]
    async fn fallible(&self, shard_id: u64) -> Result<(), &'static str> {
        let _ = shard_id;
        Err("trait failure")
    }

    #[function_metrics(name = "pending_async_trait_all", metrics(duration, calls, errors), labels(shard_id))]
    async fn pending_all(&self, shard_id: u64) -> Result<(), &'static str> {
        let _ = shard_id;
        std::future::pending::<()>().await;
        Ok(())
    }

    #[function_metrics(
        name = "panicking_async_trait_all",
        metrics(duration, calls, errors),
        labels(shard_id)
    )]
    async fn panicking_all(&self, shard_id: u64) -> Result<(), &'static str> {
        let _ = shard_id;
        panic!("async-trait panic is not a returned error")
    }
}

#[async_trait]
impl TimedOperation for TimedService {}

struct ImplementedService;

#[async_trait]
trait ImplementedOperation {
    async fn execute(&self, shard_id: u64) -> u64;
}

#[async_trait]
impl ImplementedOperation for ImplementedService {
    #[function_metrics(name = "test_async_trait_impl", labels(shard_id))]
    async fn execute(&self, shard_id: u64) -> u64 {
        tokio::time::sleep(Duration::from_millis(2)).await;
        shard_id
    }
}

struct LazyLabelService {
    captures: AtomicUsize,
}

#[async_trait]
trait LazyLabelOperation {
    async fn pending(&self);
}

#[async_trait]
impl LazyLabelOperation for LazyLabelService {
    #[function_metrics(
        name = "lazy_async_trait_label",
        labels(capture = self.captures.fetch_add(1, Ordering::Relaxed))
    )]
    async fn pending(&self) {
        std::future::pending::<()>().await;
    }
}

struct Request {
    status: String,
    region: String,
}

struct PanickingHistogram;

impl metrics::HistogramFn for PanickingHistogram {
    fn record(&self, _value: f64) {
        panic!("recorder panic");
    }
}

struct PanickingRecorder;

impl Recorder for PanickingRecorder {
    fn describe_counter(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn describe_gauge(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn describe_histogram(&self, _key: KeyName, _unit: Option<Unit>, _description: SharedString) {}

    fn register_counter(&self, _key: &Key, _metadata: &Metadata<'_>) -> Counter {
        Counter::noop()
    }

    fn register_gauge(&self, _key: &Key, _metadata: &Metadata<'_>) -> Gauge {
        Gauge::noop()
    }

    fn register_histogram(&self, _key: &Key, _metadata: &Metadata<'_>) -> Histogram {
        Histogram::from_arc(std::sync::Arc::new(PanickingHistogram))
    }
}

#[function_metrics(name = "test_fields", labels(status = request.status, request.region))]
fn process_request(request: &Request) -> usize {
    request.status.len() + request.region.len()
}

fn histogram(recorder: &DebuggingRecorder, expected_name: &str) -> (Vec<Label>, Vec<f64>) {
    recorder
        .snapshotter()
        .snapshot()
        .into_vec()
        .into_iter()
        .find_map(|(key, _, _, value)| {
            let (_, key) = key.into_parts();
            let (name, labels) = key.into_parts();
            if name.as_str() != expected_name {
                return None;
            }

            match value {
                DebugValue::Histogram(values) => {
                    Some((labels, values.into_iter().map(|value| value.into_inner()).collect()))
                }
                _ => None,
            }
        })
        .unwrap_or_else(|| panic!("missing histogram {expected_name}"))
}

fn metrics_snapshot(recorder: &DebuggingRecorder) -> std::collections::HashMap<String, (Vec<Label>, DebugValue)> {
    recorder
        .snapshotter()
        .snapshot()
        .into_vec()
        .into_iter()
        .map(|(key, _, _, value)| {
            let (_, key) = key.into_parts();
            let (name, labels) = key.into_parts();
            (name.as_str().to_owned(), (labels, value))
        })
        .collect()
}

#[test]
fn records_selected_duration_calls_and_errors() {
    let recorder = DebuggingRecorder::new();

    metrics::with_local_recorder(&recorder, || {
        assert_eq!(instrumented_result(true), Ok(()));
        assert_eq!(instrumented_result(false), Err("failed"));
    });
    let snapshot = metrics_snapshot(&recorder);
    let expected_labels = vec![Label::new("provider", "test")];
    assert!(matches!(
        snapshot.get("instrumented_result_duration_seconds"),
        Some((labels, DebugValue::Histogram(values))) if labels == &expected_labels && values.len() == 2
    ));
    assert!(matches!(
        snapshot.get("instrumented_result_calls_total"),
        Some((labels, DebugValue::Counter(2))) if labels == &expected_labels
    ));
    assert!(matches!(
        snapshot.get("instrumented_result_errors_total"),
        Some((labels, DebugValue::Counter(1))) if labels == &expected_labels
    ));
}

#[test]
fn classifies_domain_errors_without_a_duration_histogram() {
    let recorder = DebuggingRecorder::new();

    metrics::with_local_recorder(&recorder, || {
        assert_eq!(classified_result(200), 200);
        assert_eq!(classified_result(503), 503);
    });

    let snapshot = metrics_snapshot(&recorder);
    assert!(matches!(
        snapshot.get("classified_result_calls_total"),
        Some((_, DebugValue::Counter(2)))
    ));
    assert!(matches!(
        snapshot.get("classified_result_errors_total"),
        Some((_, DebugValue::Counter(1)))
    ));
    assert!(!snapshot.contains_key("classified_result_duration_seconds"));
}

#[tokio::test(flavor = "current_thread")]
async fn records_calls_for_errors_panics_and_polled_cancellation_only() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(fallible_async().await, Err("question mark"));
    assert!(std::panic::catch_unwind(panicking_result).is_err());

    let unpolled = pending_calls();
    drop(unpolled);

    {
        let mut polled = std::pin::pin!(pending_calls());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(polled.as_mut(), &mut context).is_pending());
    }

    let snapshot = metrics_snapshot(&recorder);
    assert!(matches!(
        snapshot.get("fallible_async_calls_total"),
        Some((_, DebugValue::Counter(1)))
    ));
    assert!(matches!(
        snapshot.get("fallible_async_errors_total"),
        Some((_, DebugValue::Counter(1)))
    ));
    assert!(matches!(
        snapshot.get("panicking_result_calls_total"),
        Some((_, DebugValue::Counter(1)))
    ));
    assert!(matches!(
        snapshot.get("panicking_result_errors_total"),
        Some((_, DebugValue::Counter(0)))
    ));
    assert!(matches!(
        snapshot.get("pending_calls_calls_total"),
        Some((_, DebugValue::Counter(1)))
    ));
    assert!(matches!(
        snapshot.get("pending_calls_errors_total"),
        Some((_, DebugValue::Counter(0)))
    ));
}

#[test]
fn registers_metric_descriptions_and_duration_units() {
    let recorder = DebuggingRecorder::new();
    metrics::with_local_recorder(&recorder, || assert_eq!(described_operation(), Ok(())));

    let metadata = recorder
        .snapshotter()
        .snapshot()
        .into_vec()
        .into_iter()
        .map(|(key, unit, description, _)| {
            let (_, key) = key.into_parts();
            (
                key.name().to_string(),
                (unit, description.map(|description| description.to_string())),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(
        metadata.get("described_operation_duration_seconds"),
        Some(&(
            Some(Unit::Seconds),
            Some("Duration of `described_operation` function executions.".to_owned())
        ))
    );
    assert_eq!(
        metadata.get("described_operation_calls_total"),
        Some(&(
            None,
            Some("Number of `described_operation` function executions.".to_owned())
        ))
    );
    assert_eq!(
        metadata.get("described_operation_errors_total"),
        Some(&(
            None,
            Some("Number of `described_operation` function executions that returned an error.".to_owned())
        ))
    );
}

#[test]
fn records_seconds_with_dynamic_and_static_labels() {
    let recorder = DebuggingRecorder::new();

    metrics::with_local_recorder(&recorder, || assert_eq!(timed_sync(7), 7));

    let (labels, values) = histogram(&recorder, "test_sync_duration_seconds");
    assert_eq!(
        labels,
        vec![Label::new("shard_id", "7"), Label::new("provider", "test")]
    );
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
    assert!(values[0] < 1.0);
}

#[tokio::test(flavor = "current_thread")]
async fn derives_a_name_and_handles_moved_labels() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(timed_async("shard-a".to_owned()).await, "shard-a");

    let (labels, values) = histogram(&recorder, "timed_async_duration_seconds");
    assert_eq!(labels, vec![Label::new("shard_id", "shard-a")]);
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
}

#[tokio::test(flavor = "current_thread")]
async fn times_async_trait_execution_instead_of_future_creation() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(TimedService.execute(1).await, 1);

    let (labels, values) = histogram(&recorder, "test_async_trait_duration_seconds");
    assert_eq!(labels, vec![Label::new("shard_id", "1")]);
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
}

#[tokio::test(flavor = "current_thread")]
async fn records_async_trait_calls_and_errors() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(TimedService.fallible(9).await, Err("trait failure"));

    let snapshot = metrics_snapshot(&recorder);
    let expected_labels = vec![Label::new("shard_id", "9")];
    assert!(matches!(
        snapshot.get("fallible_async_trait_duration_seconds"),
        Some((labels, DebugValue::Histogram(values))) if labels == &expected_labels && values.len() == 1
    ));
    assert!(matches!(
        snapshot.get("fallible_async_trait_calls_total"),
        Some((labels, DebugValue::Counter(1))) if labels == &expected_labels
    ));
    assert!(matches!(
        snapshot.get("fallible_async_trait_errors_total"),
        Some((labels, DebugValue::Counter(1))) if labels == &expected_labels
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn times_instrumented_async_trait_impl_methods() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(ImplementedService.execute(10).await, 10);

    let (labels, values) = histogram(&recorder, "test_async_trait_impl_duration_seconds");
    assert_eq!(labels, vec![Label::new("shard_id", "10")]);
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
}

#[test]
fn captures_explicit_and_inferred_field_labels_without_consuming_the_request() {
    let recorder = DebuggingRecorder::new();
    let request = Request {
        status: "ok".to_owned(),
        region: "us".to_owned(),
    };

    metrics::with_local_recorder(&recorder, || assert_eq!(process_request(&request), 4));

    let (labels, values) = histogram(&recorder, "test_fields_duration_seconds");
    assert_eq!(labels, vec![Label::new("status", "ok"), Label::new("region", "us")]);
    assert_eq!(values.len(), 1);
}

#[test]
fn records_sync_early_returns() {
    let recorder = DebuggingRecorder::new();

    metrics::with_local_recorder(&recorder, || assert_eq!(early_sync(true), Err("early return")));

    let (_, values) = histogram(&recorder, "early_sync_duration_seconds");
    assert_eq!(values.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn records_async_question_mark_returns() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(question_mark_async().await, Err("question mark"));

    let (_, values) = histogram(&recorder, "question_mark_async_duration_seconds");
    assert_eq!(values.len(), 1);
}

#[test]
fn records_panicking_functions() {
    let recorder = DebuggingRecorder::new();

    metrics::with_local_recorder(&recorder, || {
        assert!(std::panic::catch_unwind(panicking_sync).is_err());
    });

    let (_, values) = histogram(&recorder, "panicking_sync_duration_seconds");
    assert_eq!(values.len(), 1);
}

#[test]
fn recorder_panic_during_unwind_does_not_abort() {
    const CHILD_PROCESS: &str = "FUNCTION_METRICS_UNWIND_CHILD";

    if std::env::var_os(CHILD_PROCESS).is_some() {
        metrics::with_local_recorder(&PanickingRecorder, || {
            assert!(std::panic::catch_unwind(panicking_sync).is_err());
        });
        return;
    }

    let status = std::process::Command::new(std::env::current_exe().expect("test executable must exist"))
        .args(["--exact", "recorder_panic_during_unwind_does_not_abort"])
        .env(CHILD_PROCESS, "1")
        .status()
        .expect("child test process must start");

    assert!(status.success(), "child process aborted with {status}");
}

#[tokio::test(flavor = "current_thread")]
async fn records_cancelled_futures_after_polling_starts() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    {
        let mut future = std::pin::pin!(pending_async());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(future.as_mut(), &mut context).is_pending());
    }

    let (_, values) = histogram(&recorder, "cancelled_async_duration_seconds");
    assert_eq!(values.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn records_cancelled_async_trait_futures_with_labels() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    {
        let mut future = std::pin::pin!(TimedService.pending(17));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(future.as_mut(), &mut context).is_pending());
    }

    let (labels, values) = histogram(&recorder, "cancelled_async_trait_duration_seconds");
    assert_eq!(labels, vec![Label::new("shard_id", "17")]);
    assert_eq!(values.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn records_async_trait_calls_for_panics_and_polled_cancellation_only() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    let unpolled = TimedService.pending_all(18);
    drop(unpolled);

    {
        let mut polled = std::pin::pin!(TimedService.pending_all(18));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(polled.as_mut(), &mut context).is_pending());
    }

    {
        let mut panicking = std::pin::pin!(TimedService.panicking_all(19));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                std::future::Future::poll(panicking.as_mut(), &mut context)
            }))
            .is_err()
        );
    }

    let snapshot = metrics_snapshot(&recorder);
    for (name, shard_id) in [("pending_async_trait_all", "18"), ("panicking_async_trait_all", "19")] {
        let expected_labels = vec![Label::new("shard_id", shard_id)];
        assert!(matches!(
            snapshot.get(&format!("{name}_duration_seconds")),
            Some((labels, DebugValue::Histogram(values))) if labels == &expected_labels && values.len() == 1
        ));
        assert!(matches!(
            snapshot.get(&format!("{name}_calls_total")),
            Some((labels, DebugValue::Counter(1))) if labels == &expected_labels
        ));
        assert!(matches!(
            snapshot.get(&format!("{name}_errors_total")),
            Some((labels, DebugValue::Counter(0))) if labels == &expected_labels
        ));
    }
}

#[tokio::test(flavor = "current_thread")]
async fn defers_async_trait_labels_until_first_poll() {
    let service = LazyLabelService {
        captures: AtomicUsize::new(0),
    };
    let mut future = std::pin::pin!(service.pending());

    assert_eq!(service.captures.load(Ordering::Relaxed), 0);

    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    assert!(std::future::Future::poll(future.as_mut(), &mut context).is_pending());
    assert_eq!(service.captures.load(Ordering::Relaxed), 1);
}
