use std::time::Duration;

use async_trait::async_trait;
use function_metrics::function_metrics;
use metrics::Label;
use metrics_util::debugging::{DebugValue, DebuggingRecorder};

#[function_metrics(name = "test_sync", labels(chain_id, provider = "test"))]
fn timed_sync(chain_id: u64) -> u64 {
    std::thread::sleep(Duration::from_millis(2));
    chain_id
}

#[function_metrics(labels(chain_id))]
async fn timed_async(chain_id: String) -> String {
    tokio::time::sleep(Duration::from_millis(2)).await;
    chain_id
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

struct TimedService;

#[async_trait]
trait TimedOperation {
    #[function_metrics(name = "test_async_trait", labels(chain_id))]
    async fn execute(&self, chain_id: u64) -> u64 {
        tokio::time::sleep(Duration::from_millis(2)).await;
        chain_id
    }
}

#[async_trait]
impl TimedOperation for TimedService {}

struct ImplementedService;

#[async_trait]
trait ImplementedOperation {
    async fn execute(&self, chain_id: u64) -> u64;
}

#[async_trait]
impl ImplementedOperation for ImplementedService {
    #[function_metrics(name = "test_async_trait_impl", labels(chain_id))]
    async fn execute(&self, chain_id: u64) -> u64 {
        tokio::time::sleep(Duration::from_millis(2)).await;
        chain_id
    }
}

struct Request {
    status: String,
    region: String,
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

#[test]
fn records_seconds_with_dynamic_and_static_labels() {
    let recorder = DebuggingRecorder::new();

    metrics::with_local_recorder(&recorder, || assert_eq!(timed_sync(8453), 8453));

    let (labels, values) = histogram(&recorder, "test_sync_duration_seconds");
    assert_eq!(
        labels,
        vec![Label::new("chain_id", "8453"), Label::new("provider", "test")]
    );
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
    assert!(values[0] < 1.0);
}

#[tokio::test(flavor = "current_thread")]
async fn derives_a_name_and_handles_moved_labels() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(timed_async("solana".to_owned()).await, "solana");

    let (labels, values) = histogram(&recorder, "timed_async_duration_seconds");
    assert_eq!(labels, vec![Label::new("chain_id", "solana")]);
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
}

#[tokio::test(flavor = "current_thread")]
async fn times_async_trait_execution_instead_of_future_creation() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(TimedService.execute(1).await, 1);

    let (labels, values) = histogram(&recorder, "test_async_trait_duration_seconds");
    assert_eq!(labels, vec![Label::new("chain_id", "1")]);
    assert_eq!(values.len(), 1);
    assert!(values[0] >= 0.001);
}

#[tokio::test(flavor = "current_thread")]
async fn times_instrumented_async_trait_impl_methods() {
    let recorder = DebuggingRecorder::new();
    let _guard = metrics::set_default_local_recorder(&recorder);

    assert_eq!(ImplementedService.execute(10).await, 10);

    let (labels, values) = histogram(&recorder, "test_async_trait_impl_duration_seconds");
    assert_eq!(labels, vec![Label::new("chain_id", "10")]);
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
