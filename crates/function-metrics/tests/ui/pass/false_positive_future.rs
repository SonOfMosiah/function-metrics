use function_metrics::function_metrics;

struct Future;

#[function_metrics]
fn construct_future() -> Future {
    Future
}

fn main() {
    let _ = construct_future();
}
