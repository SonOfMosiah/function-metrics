use async_trait::async_trait;
use function_metrics::function_metrics;

#[function_metrics(labels(label = __function_metrics_label_0))]
fn sync_collision(
    __function_metrics_guard: usize,
    __function_metrics_result: usize,
    __function_metrics_label_0: &str,
) -> usize {
    __function_metrics_guard + __function_metrics_result + __function_metrics_label_0.len()
}

#[function_metrics]
async fn async_collision(__function_metrics_guard: usize, __function_metrics_result: usize) -> usize {
    __function_metrics_guard + __function_metrics_result
}

struct Service;

#[async_trait]
trait Operation {
    #[function_metrics]
    async fn execute(&self, __function_metrics_future: usize) -> usize {
        __function_metrics_future
    }
}

#[async_trait]
impl Operation for Service {}

fn main() {
    assert_eq!(sync_collision(1, 2, "abc"), 6);
    let _ = async_collision(1, 2);
    let _ = Service.execute(3);
}
