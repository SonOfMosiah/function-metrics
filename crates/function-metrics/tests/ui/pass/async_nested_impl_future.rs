use function_metrics::function_metrics;

#[function_metrics]
async fn construct_future() -> impl std::future::Future<Output = u64> {
    async { 42 }
}

fn main() {
    let _ = construct_future();
}
