use function_metrics::function_metrics;

#[function_metrics]
fn construct_future() -> impl std::future::Future<Output = u64> {
    async { 42 }
}

fn main() {}
